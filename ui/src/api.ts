import {
  BookMetadata,
  BookProject,
  CleanerOptions,
  ExportConfig,
  RenderArtifact,
  SystemInfoResponse,
  TtsConfig,
  VoicePreviewResponse,
} from './types';

const API_BASE = 'http://127.0.0.1:14200';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
    };
  }
}

function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__);
}

export async function fetchSystemInfo(): Promise<SystemInfoResponse> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<SystemInfoResponse>('get_system_info');
  }
  const res = await fetch(`${API_BASE}/api/system`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function fetchProject(): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('get_project');
  }
  const res = await fetch(`${API_BASE}/api/project`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function loadSampleProject(): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('load_sample_project');
  }
  const res = await fetch(`${API_BASE}/api/project/sample`, { method: 'POST' });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function uploadBookFile(file: File): Promise<BookProject> {
  const form = new FormData();
  form.append('file', file);
  const res = await fetch(`${API_BASE}/api/project/upload`, {
    method: 'POST',
    body: form,
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function importProjectFromPath(path: string): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('import_project_from_path', { path });
  }
  const res = await fetch(`${API_BASE}/api/project/import-path`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function saveMetadata(metadata: BookMetadata): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('update_metadata', { metadata });
  }
  const res = await fetch(`${API_BASE}/api/project/metadata`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(metadata),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function applyCleanerOptions(options: CleanerOptions): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('update_cleaner_options', { options });
  }
  const res = await fetch(`${API_BASE}/api/project/cleaner`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(options),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function updateChapter(params: {
  chapter_id: string;
  title?: string;
  cleaned_text?: string;
  enabled?: boolean;
}): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('update_chapter', {
      chapterId: params.chapter_id,
      title: params.title ?? null,
      cleanedText: params.cleaned_text ?? null,
      enabled: params.enabled ?? null,
    });
  }
  const res = await fetch(`${API_BASE}/api/project/chapter/update`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function mergeChapterWithNext(chapter_id: string): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('merge_chapter', {
      chapterId: chapter_id,
    });
  }
  const res = await fetch(`${API_BASE}/api/project/chapter/merge`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ chapter_id }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function addCustomChapter(title: string, text: string): Promise<BookProject> {
  const res = await fetch(`${API_BASE}/api/project/chapter/add`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, text }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function saveTtsConfig(tts_config: TtsConfig): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('update_tts_config', {
      ttsConfig: tts_config,
    });
  }
  const res = await fetch(`${API_BASE}/api/project/tts`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(tts_config),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function saveExportConfig(export_config: ExportConfig): Promise<BookProject> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<BookProject>('update_export_config', {
      exportConfig: export_config,
    });
  }
  const res = await fetch(`${API_BASE}/api/project/export-config`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(export_config),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function previewVoice(
  text: string,
  tts_config: TtsConfig
): Promise<VoicePreviewResponse> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<VoicePreviewResponse>('preview_voice', {
      text,
      ttsConfig: tts_config,
    });
  }
  const res = await fetch(`${API_BASE}/api/tts/preview`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text, tts_config }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function startAudiobookRender(): Promise<RenderArtifact> {
  if (isTauri()) {
    return window.__TAURI_INTERNALS__!.invoke<RenderArtifact>('start_render');
  }
  const res = await fetch(`${API_BASE}/api/render/start`, { method: 'POST' });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

