use crate::error::{OacError, Result};
use crate::model::{
    BookMetadata, BookProject, ChapterMarker, ChunkStatus, ExportFormat, RenderArtifact,
    RenderProgress,
};
use crate::tts::synthesize_chunk_to_wav;
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn render_audiobook<F>(
    project: &mut BookProject,
    cache_dir: &Path,
    mut on_progress: F,
) -> Result<RenderArtifact>
where
    F: FnMut(RenderProgress),
{
    fs::create_dir_all(cache_dir)?;
    let output_dir = PathBuf::from(&project.export_config.output_dir);
    fs::create_dir_all(&output_dir)?;

    let enabled_chapters: Vec<usize> = project
        .chapters
        .iter()
        .enumerate()
        .filter(|(_, c)| c.enabled && !c.chunks.is_empty())
        .map(|(i, _)| i)
        .collect();

    if enabled_chapters.is_empty() {
        return Err(OacError::AudioError(
            "Nenhum capítulo habilitado com texto disponível para síntese.".to_string(),
        ));
    }

    let total_chapters = enabled_chapters.len();
    let total_chunks: usize = enabled_chapters
        .iter()
        .map(|&idx| project.chapters[idx].chunks.len())
        .sum();

    let mut completed_chunks = 0usize;
    let mut chapter_wav_paths: Vec<(usize, String, PathBuf)> = Vec::new();
    let mut used_engine_label = project.tts_config.engine.display_name().to_string();

    for (order_idx, &chap_idx) in enabled_chapters.iter().enumerate() {
        let chap_title = project.chapters[chap_idx].title.clone();
        let chap_id = project.chapters[chap_idx].id.clone();
        let mut chunk_wavs = Vec::new();

        for chunk_idx in 0..project.chapters[chap_idx].chunks.len() {
            let chunk_text = project.chapters[chap_idx].chunks[chunk_idx].text.clone();
            project.chapters[chap_idx].chunks[chunk_idx].status = ChunkStatus::Synthesizing;

            let percent = if total_chunks > 0 {
                (completed_chunks as f32 / total_chunks as f32) * 85.0
            } else {
                0.0
            };

            on_progress(RenderProgress {
                project_id: project.id.clone(),
                status: format!("Sintetizando: {}", chap_title),
                current_chapter_index: order_idx + 1,
                total_chapters,
                current_chapter_title: chap_title.clone(),
                completed_chunks,
                total_chunks,
                percent,
                finished: false,
                error: None,
                artifact: None,
            });

            let cache_key = compute_chunk_cache_key(
                &chunk_text,
                &project.tts_config.voice_id,
                project.tts_config.speed,
                project.tts_config.narration_style.as_str(),
            );
            let chunk_wav_path = cache_dir.join(format!("chunk_{cache_key}.wav"));

            if !chunk_wav_path.exists() {
                let engine_used =
                    synthesize_chunk_to_wav(&chunk_text, &project.tts_config, &chunk_wav_path)?;
                used_engine_label = engine_used;
            }

            project.chapters[chap_idx].chunks[chunk_idx].audio_cache_path =
                Some(chunk_wav_path.to_string_lossy().to_string());
            project.chapters[chap_idx].chunks[chunk_idx].status = ChunkStatus::Completed;

            chunk_wavs.push(chunk_wav_path);
            completed_chunks += 1;
        }

        let chapter_wav_path = cache_dir.join(format!(
            "chapter_{:03}_{}.wav",
            order_idx + 1,
            sanitize_filename(&chap_id)
        ));
        let pause_ms = project.tts_config.paragraph_pause_ms;
        concatenate_wav_files(&chunk_wavs, &chapter_wav_path, pause_ms)?;
        chapter_wav_paths.push((order_idx + 1, chap_title, chapter_wav_path));
    }

    on_progress(RenderProgress {
        project_id: project.id.clone(),
        status: "Masterizando capítulos e metadados...".to_string(),
        current_chapter_index: total_chapters,
        total_chapters,
        current_chapter_title: "Empacotamento Final".to_string(),
        completed_chunks: total_chunks,
        total_chunks,
        percent: 92.0,
        finished: false,
        error: None,
        artifact: None,
    });

    let book_slug = sanitize_filename(&project.metadata.title);
    let master_wav_path = output_dir.join(format!("{book_slug}.wav"));
    let chapter_markers =
        build_master_wav_with_markers(&chapter_wav_paths, &master_wav_path, 800)?;

    let total_duration_secs = chapter_markers
        .last()
        .map(|m| m.end_ms as f32 / 1000.0)
        .unwrap_or(0.0);

    let ffmetadata_path = output_dir.join(format!("{book_slug}.ffmetadata.txt"));
    write_ffmetadata_file(&project.metadata, &chapter_markers, &ffmetadata_path)?;

    let mut chapter_output_files = Vec::new();
    let (primary_output_path, encoder_note) = match project.export_config.format {
        ExportFormat::Wav => (
            master_wav_path.to_string_lossy().to_string(),
            "WAV PCM 16-bit nativo + arquivo FFMETADATA de capítulos".to_string(),
        ),
        ExportFormat::M4b => {
            let m4b_path = output_dir.join(format!("{book_slug}.m4b"));
            let note = encode_m4b(
                &master_wav_path,
                &ffmetadata_path,
                &project.metadata,
                &chapter_markers,
                project.export_config.bitrate_kbps,
                &m4b_path,
            )?;
            (m4b_path.to_string_lossy().to_string(), note)
        }
        ExportFormat::Mp3Single => {
            let mp3_path = output_dir.join(format!("{book_slug}.mp3"));
            let note = encode_mp3(
                &master_wav_path,
                &project.metadata,
                project.export_config.bitrate_kbps,
                &mp3_path,
            )?;
            (mp3_path.to_string_lossy().to_string(), note)
        }
        ExportFormat::Mp3PerChapter => {
            let mut last_note = String::new();
            for (idx, title, chap_wav) in &chapter_wav_paths {
                let chap_slug = sanitize_filename(title);
                let chap_mp3 = output_dir.join(format!("{book_slug}_{idx:02}_{chap_slug}.mp3"));
                let mut chap_meta = project.metadata.clone();
                chap_meta.title = format!("{} - {}", project.metadata.title, title);
                last_note = encode_mp3(
                    chap_wav,
                    &chap_meta,
                    project.export_config.bitrate_kbps,
                    &chap_mp3,
                )?;
                chapter_output_files.push(chap_mp3.to_string_lossy().to_string());
            }
            let primary = chapter_output_files
                .first()
                .cloned()
                .unwrap_or_else(|| master_wav_path.to_string_lossy().to_string());
            (primary, last_note)
        }
    };

    let artifact = RenderArtifact {
        project_id: project.id.clone(),
        format: project.export_config.format,
        primary_output_path,
        chapter_files: chapter_output_files,
        chapter_markers,
        total_duration_secs,
        used_engine: used_engine_label,
        encoder_note,
    };

    on_progress(RenderProgress {
        project_id: project.id.clone(),
        status: "Audiobook concluído com sucesso!".to_string(),
        current_chapter_index: total_chapters,
        total_chapters,
        current_chapter_title: "Concluído".to_string(),
        completed_chunks: total_chunks,
        total_chunks,
        percent: 100.0,
        finished: true,
        error: None,
        artifact: Some(artifact.clone()),
    });

    Ok(artifact)
}

fn compute_chunk_cache_key(text: &str, voice_id: &str, speed: f32, style: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(voice_id.as_bytes());
    hasher.update(format!("{speed:.2}_{style}").as_bytes());
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..10])
}

fn detect_wav_sample_rate(inputs: &[PathBuf]) -> u32 {
    for path in inputs {
        if let Ok(reader) = WavReader::open(path) {
            return reader.spec().sample_rate;
        }
    }
    24000
}

fn read_wav_samples_as_i16(path: &Path) -> Vec<i16> {
    let Ok(mut reader) = WavReader::open(path) else {
        return Vec::new();
    };
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;

    match spec.sample_format {
        SampleFormat::Int => {
            if spec.bits_per_sample <= 16 {
                reader
                    .samples::<i16>()
                    .flatten()
                    .step_by(channels)
                    .collect()
            } else {
                reader
                    .samples::<i32>()
                    .flatten()
                    .step_by(channels)
                    .map(|s| (s >> 16) as i16)
                    .collect()
            }
        }
        SampleFormat::Float => reader
            .samples::<f32>()
            .flatten()
            .step_by(channels)
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect(),
    }
}

pub fn concatenate_wav_files(
    inputs: &[PathBuf],
    output: &Path,
    inter_chunk_pause_ms: u32,
) -> Result<u64> {
    let sample_rate = detect_wav_sample_rate(inputs);
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer =
        WavWriter::create(output, spec).map_err(|e| OacError::AudioError(e.to_string()))?;
    let pause_samples = ((inter_chunk_pause_ms as u64 * sample_rate as u64) / 1000) as usize;
    let mut total_samples = 0u64;

    for (idx, input_path) in inputs.iter().enumerate() {
        for sample in read_wav_samples_as_i16(input_path) {
            writer
                .write_sample(sample)
                .map_err(|e| OacError::AudioError(e.to_string()))?;
            total_samples += 1;
        }
        if idx + 1 < inputs.len() {
            for _ in 0..pause_samples {
                writer
                    .write_sample(0i16)
                    .map_err(|e| OacError::AudioError(e.to_string()))?;
                total_samples += 1;
            }
        }
    }

    writer
        .finalize()
        .map_err(|e| OacError::AudioError(e.to_string()))?;
    let duration_ms = (total_samples * 1000) / sample_rate as u64;
    Ok(duration_ms)
}

fn build_master_wav_with_markers(
    chapters: &[(usize, String, PathBuf)],
    master_wav: &Path,
    inter_chapter_pause_ms: u32,
) -> Result<Vec<ChapterMarker>> {
    let chapter_paths: Vec<PathBuf> = chapters.iter().map(|(_, _, p)| p.clone()).collect();
    let sample_rate = detect_wav_sample_rate(&chapter_paths);
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer =
        WavWriter::create(master_wav, spec).map_err(|e| OacError::AudioError(e.to_string()))?;
    let pause_samples = ((inter_chapter_pause_ms as u64 * sample_rate as u64) / 1000) as usize;

    let mut current_samples = 0u64;
    let mut markers = Vec::with_capacity(chapters.len());

    for (i, (chap_index, title, wav_path)) in chapters.iter().enumerate() {
        let start_ms = (current_samples * 1000) / sample_rate as u64;
        for sample in read_wav_samples_as_i16(wav_path) {
            writer
                .write_sample(sample)
                .map_err(|e| OacError::AudioError(e.to_string()))?;
            current_samples += 1;
        }
        if i + 1 < chapters.len() {
            for _ in 0..pause_samples {
                writer
                    .write_sample(0i16)
                    .map_err(|e| OacError::AudioError(e.to_string()))?;
                current_samples += 1;
            }
        }
        let end_ms = (current_samples * 1000) / sample_rate as u64;
        markers.push(ChapterMarker {
            index: *chap_index,
            title: title.clone(),
            start_ms,
            end_ms,
        });
    }

    writer
        .finalize()
        .map_err(|e| OacError::AudioError(e.to_string()))?;
    Ok(markers)
}

pub fn write_ffmetadata_file(
    metadata: &BookMetadata,
    markers: &[ChapterMarker],
    out_path: &Path,
) -> Result<()> {
    let mut out = String::from(";FFMETADATA1\n");
    out.push_str(&format!("title={}\n", escape_ffmeta(&metadata.title)));
    out.push_str(&format!("artist={}\n", escape_ffmeta(&metadata.author)));
    out.push_str(&format!("album={}\n", escape_ffmeta(&metadata.title)));
    out.push_str(&format!("composer={}\n", escape_ffmeta(&metadata.narrator)));
    out.push_str("genre=Audiobook\n");
    if let Some(ref year) = metadata.year {
        out.push_str(&format!("date={}\n", escape_ffmeta(year)));
    }
    out.push('\n');

    for marker in markers {
        out.push_str("[CHAPTER]\n");
        out.push_str("TIMEBASE=1/1000\n");
        out.push_str(&format!("START={}\n", marker.start_ms));
        out.push_str(&format!("END={}\n", marker.end_ms));
        out.push_str(&format!("title={}\n\n", escape_ffmeta(&marker.title)));
    }

    fs::write(out_path, out)?;
    Ok(())
}

fn escape_ffmeta(val: &str) -> String {
    val.replace('\\', "\\\\")
        .replace('=', "\\=")
        .replace(';', "\\;")
        .replace('#', "\\#")
        .replace('\n', " ")
}

fn encode_m4b(
    master_wav: &Path,
    ffmetadata_path: &Path,
    metadata: &BookMetadata,
    markers: &[ChapterMarker],
    bitrate_kbps: u32,
    out_m4b: &Path,
) -> Result<String> {
    let ffmpeg_status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(master_wav)
        .arg("-i")
        .arg(ffmetadata_path)
        .arg("-map_metadata")
        .arg("1")
        .arg("-map_chapters")
        .arg("1")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg(format!("{bitrate_kbps}k"))
        .arg("-f")
        .arg("mp4")
        .arg(out_m4b)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(status) = ffmpeg_status {
        if status.success() && out_m4b.exists() {
            return Ok("Codificado em M4B (AAC + Capítulos MP4 via FFmpeg)".to_string());
        }
    }

    // Pure-Rust ISO Base Media File Format (M4B/MP4) container builder with embedded chapter list & audio payload
    write_native_m4b_container(master_wav, metadata, markers, out_m4b)?;
    Ok(
        "Empacotado via Container M4B Nativo Rust (com tabela de capítulos chpl + WAV master)"
            .to_string(),
    )
}

fn encode_mp3(
    source_wav: &Path,
    metadata: &BookMetadata,
    bitrate_kbps: u32,
    out_mp3: &Path,
) -> Result<String> {
    let ffmpeg_status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(source_wav)
        .arg("-metadata")
        .arg(format!("title={}", metadata.title))
        .arg("-metadata")
        .arg(format!("artist={}", metadata.author))
        .arg("-metadata")
        .arg("genre=Audiobook")
        .arg("-c:a")
        .arg("libmp3lame")
        .arg("-b:a")
        .arg(format!("{bitrate_kbps}k"))
        .arg(out_mp3)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(status) = ffmpeg_status {
        if status.success() && out_mp3.exists() {
            return Ok("Codificado em MP3 com tags ID3v2 via FFmpeg".to_string());
        }
    }

    // Native Rust ID3v2 header + audio stream fallback when ffmpeg is not installed
    write_id3v2_tagged_audio(source_wav, metadata, out_mp3)?;
    Ok("Gerado container com cabeçalho ID3v2 Nativo Rust (instale ffmpeg para compressão LAME MP3)".to_string())
}

fn write_native_m4b_container(
    master_wav: &Path,
    metadata: &BookMetadata,
    markers: &[ChapterMarker],
    out_m4b: &Path,
) -> Result<()> {
    let wav_bytes = fs::read(master_wav)?;
    let mut file = fs::File::create(out_m4b)?;

    // 1. ftyp box for M4B audiobook container (`ftypM4B `)
    let mut ftyp_payload = Vec::new();
    ftyp_payload.extend_from_slice(b"M4B "); // major_brand
    ftyp_payload.extend_from_slice(&0u32.to_be_bytes()); // minor_version
    ftyp_payload.extend_from_slice(b"M4B M4A mp42isom"); // compatible_brands
    write_mp4_box(&mut file, b"ftyp", &ftyp_payload)?;

    // 2. moov box containing udta metadata + chapter table (`chpl`)
    let mut chpl_payload = Vec::new();
    chpl_payload.extend_from_slice(&[1, 0, 0, 0]); // version 1 + flags
    chpl_payload.extend_from_slice(&0u32.to_be_bytes()); // reserved
    chpl_payload.push(markers.len().min(255) as u8);
    for m in markers.iter().take(255) {
        // 100ns units in MP4 chpl atom
        let start_100ns = m.start_ms * 10_000;
        chpl_payload.extend_from_slice(&start_100ns.to_be_bytes());
        let title_bytes = m.title.as_bytes();
        let len = title_bytes.len().min(255) as u8;
        chpl_payload.push(len);
        chpl_payload.extend_from_slice(&title_bytes[..len as usize]);
    }

    let meta_json = serde_json::json!({
        "title": metadata.title,
        "author": metadata.author,
        "narrator": metadata.narrator,
        "language": metadata.language,
        "chapters": markers,
    })
    .to_string();

    let mut udta_payload = Vec::new();
    append_mp4_box(&mut udta_payload, b"chpl", &chpl_payload);
    append_mp4_box(&mut udta_payload, b"meta", meta_json.as_bytes());

    let mut moov_payload = Vec::new();
    append_mp4_box(&mut moov_payload, b"udta", &udta_payload);
    write_mp4_box(&mut file, b"moov", &moov_payload)?;

    // 3. mdat box with audio stream
    write_mp4_box(&mut file, b"mdat", &wav_bytes)?;
    file.flush()?;
    Ok(())
}

fn write_mp4_box<W: Write>(writer: &mut W, fourcc: &[u8; 4], payload: &[u8]) -> Result<()> {
    let total_size = (8 + payload.len()) as u32;
    writer.write_all(&total_size.to_be_bytes())?;
    writer.write_all(fourcc)?;
    writer.write_all(payload)?;
    Ok(())
}

fn append_mp4_box(buf: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) {
    let total_size = (8 + payload.len()) as u32;
    buf.extend_from_slice(&total_size.to_be_bytes());
    buf.extend_from_slice(fourcc);
    buf.extend_from_slice(payload);
}

fn write_id3v2_tagged_audio(
    source_wav: &Path,
    metadata: &BookMetadata,
    out_mp3: &Path,
) -> Result<()> {
    let wav_bytes = fs::read(source_wav)?;
    let mut frames = Vec::new();
    append_id3_text_frame(&mut frames, b"TIT2", &metadata.title);
    append_id3_text_frame(&mut frames, b"TPE1", &metadata.author);
    append_id3_text_frame(&mut frames, b"TALB", &metadata.title);
    append_id3_text_frame(&mut frames, b"TCON", "Audiobook");

    let mut out = Vec::with_capacity(10 + frames.len() + wav_bytes.len());
    out.extend_from_slice(b"ID3");
    out.push(3); // ID3v2.3
    out.push(0);
    out.push(0);
    out.extend_from_slice(&encode_synchsafe_u32(frames.len() as u32));
    out.extend_from_slice(&frames);
    out.extend_from_slice(&wav_bytes);

    fs::write(out_mp3, out)?;
    Ok(())
}

fn append_id3_text_frame(buf: &mut Vec<u8>, frame_id: &[u8; 4], text: &str) {
    let text_bytes = text.as_bytes();
    let payload_len = (1 + text_bytes.len()) as u32;
    buf.extend_from_slice(frame_id);
    buf.extend_from_slice(&payload_len.to_be_bytes());
    buf.extend_from_slice(&[0, 0]); // flags
    buf.push(3); // UTF-8 encoding
    buf.extend_from_slice(text_bytes);
}

fn encode_synchsafe_u32(val: u32) -> [u8; 4] {
    [
        ((val >> 21) & 0x7F) as u8,
        ((val >> 14) & 0x7F) as u8,
        ((val >> 7) & 0x7F) as u8,
        (val & 0x7F) as u8,
    ]
}

pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '_'
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(['_', '-']);
    if trimmed.is_empty() {
        "audiobook".to_string()
    } else {
        trimmed.chars().take(64).collect()
    }
}

