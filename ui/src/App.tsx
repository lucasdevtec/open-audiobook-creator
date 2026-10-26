import React, { useEffect, useState } from 'react';
import {
  BookOpen,
  CheckCircle2,
  Cpu,
  Download,
  FileText,
  FolderOpen,
  Headphones,
  Layers,
  Merge,
  Mic,
  Play,
  Plus,
  RefreshCw,
  Settings2,
  Sparkles,
  Upload,
  Volume2,
  Wand2,
} from 'lucide-react';
import {
  addCustomChapter,
  applyCleanerOptions,
  fetchProject,
  fetchSystemInfo,
  loadSampleProject,
  mergeChapterWithNext,
  openOutputDirectory,
  pickOutputDirectory,
  previewVoice,
  saveExportConfig,
  saveMetadata,
  saveTtsConfig,
  startAudiobookRender,
  updateChapter,
  uploadBookFile,
} from './api';
import {
  BookMetadata,
  BookProject,
  CleanerOptions,
  ExportConfig,
  NarrationStyle,
  RenderArtifact,
  SystemInfoResponse,
  TtsConfig,
  TtsEngineKind,
} from './types';

type StudioTab = 'import' | 'chapters' | 'voices' | 'render';

export default function App() {
  const [activeTab, setActiveTab] = useState<StudioTab>('import');
  const [project, setProject] = useState<BookProject | null>(null);
  const [systemInfo, setSystemInfo] = useState<SystemInfoResponse | null>(null);
  const [selectedChapterId, setSelectedChapterId] = useState<string | null>(null);
  const [chapterEditTitle, setChapterEditTitle] = useState('');
  const [chapterEditText, setChapterEditText] = useState('');
  const [showRawDiff, setShowRawDiff] = useState(false);

  const [previewSampleText, setPreviewSampleText] = useState(
    'Um silêncio de três partes.\n\nNoite outra vez. A Pousada Marco do Percurso estava em silêncio, e era um silêncio em três partes. Era profundo e amplo como o fim do outono... som de flor colhida – do homem que espera a morte.'
  );
  const [previewAudioUrl, setPreviewAudioUrl] = useState<string | null>(null);
  const [previewEngineLabel, setPreviewEngineLabel] = useState<string | null>(null);
  const [isPreviewLoading, setIsPreviewLoading] = useState(false);

  const [isRendering, setIsRendering] = useState(false);
  const [renderResult, setRenderResult] = useState<RenderArtifact | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    initStudio();
  }, []);

  async function initStudio() {
    try {
      const [sys, proj] = await Promise.all([fetchSystemInfo(), fetchProject()]);
      setSystemInfo(sys);
      setProject(proj);
      if (proj.chapters.length > 0) {
        selectChapter(proj, proj.chapters[0].id);
      }
    } catch (err: any) {
      setErrorMessage(
        err?.message || 'Não foi possível inicializar o motor local do Open Audiobook Creator.'
      );
    }
  }

  function selectChapter(proj: BookProject, chapterId: string) {
    setSelectedChapterId(chapterId);
    const ch = proj.chapters.find((c) => c.id === chapterId);
    if (ch) {
      setChapterEditTitle(ch.title);
      setChapterEditText(ch.cleaned_text);
    }
  }

  async function handleFileUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setErrorMessage(null);
    setStatusMessage(`Importando e higienizando "${file.name}"...`);
    try {
      const updated = await uploadBookFile(file);
      setProject(updated);
      if (updated.chapters.length > 0) {
        selectChapter(updated, updated.chapters[0].id);
      }
      setStatusMessage(
        `Livro importado com sucesso: ${updated.chapters.length} capítulos extraídos.`
      );
      setActiveTab('chapters');
    } catch (err: any) {
      setErrorMessage(err?.message || 'Erro ao importar arquivo.');
    }
  }

  async function handleLoadSample() {
    setErrorMessage(null);
    setStatusMessage('Gerando livro PDF de demonstração e aplicando limpeza...');
    try {
      const updated = await loadSampleProject();
      setProject(updated);
      if (updated.chapters.length > 0) {
        selectChapter(updated, updated.chapters[0].id);
      }
      setStatusMessage('Livro PDF de exemplo carregado!');
    } catch (err: any) {
      setErrorMessage(err?.message || 'Erro ao carregar exemplo.');
    }
  }

  async function handleMetadataChange(nextMeta: BookMetadata) {
    if (!project) return;
    setProject({ ...project, metadata: nextMeta });
    try {
      const updated = await saveMetadata(nextMeta);
      setProject(updated);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Falha ao salvar metadados.');
    }
  }

  async function handleCleanerChange(nextCleaner: CleanerOptions) {
    if (!project) return;
    setStatusMessage('Reaplicando regras de limpeza inteligente nos capítulos...');
    try {
      const updated = await applyCleanerOptions(nextCleaner);
      setProject(updated);
      if (selectedChapterId) {
        selectChapter(updated, selectedChapterId);
      }
      setStatusMessage('Limpeza reaplicada com sucesso.');
    } catch (err: any) {
      setErrorMessage(err?.message || 'Erro ao reaplicar limpeza.');
    }
  }

  async function handleSaveChapterEdits() {
    if (!project || !selectedChapterId) return;
    try {
      const updated = await updateChapter({
        chapter_id: selectedChapterId,
        title: chapterEditTitle,
        cleaned_text: chapterEditText,
      });
      setProject(updated);
      setStatusMessage('Alterações do capítulo salvas e sentenças recalculadas.');
    } catch (err: any) {
      setErrorMessage(err?.message || 'Falha ao salvar capítulo.');
    }
  }

  async function handleToggleChapter(chapterId: string, enabled: boolean) {
    try {
      const updated = await updateChapter({ chapter_id: chapterId, enabled });
      setProject(updated);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Erro ao alternar capítulo.');
    }
  }

  async function handleMergeChapter(chapterId: string) {
    try {
      const updated = await mergeChapterWithNext(chapterId);
      setProject(updated);
      selectChapter(updated, chapterId);
      setStatusMessage('Capítulo mesclado com o próximo com sucesso.');
    } catch (err: any) {
      setErrorMessage(err?.message || 'Erro ao mesclar capítulos.');
    }
  }

  async function handleAddChapter() {
    try {
      const updated = await addCustomChapter(
        'Novo Capítulo Extra',
        'Digite ou cole aqui o conteúdo do novo capítulo para síntese de voz.'
      );
      setProject(updated);
      const last = updated.chapters[updated.chapters.length - 1];
      if (last) {
        selectChapter(updated, last.id);
      }
    } catch (err: any) {
      setErrorMessage(err?.message || 'Erro ao adicionar capítulo.');
    }
  }

  async function handleTtsConfigChange(nextTts: TtsConfig) {
    if (!project) return;
    setProject({ ...project, tts_config: nextTts });
    try {
      const updated = await saveTtsConfig(nextTts);
      setProject(updated);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Falha ao atualizar configuração TTS.');
    }
  }

  async function handleExportConfigChange(nextExport: ExportConfig) {
    if (!project) return;
    setProject({ ...project, export_config: nextExport });
    try {
      const updated = await saveExportConfig(nextExport);
      setProject(updated);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Falha ao salvar opções de exportação.');
    }
  }

  async function handlePickOutputFolder() {
    if (!project) return;
    try {
      const selected = await pickOutputDirectory(project.export_config.output_dir);
      if (selected) {
        await handleExportConfigChange({
          ...project.export_config,
          output_dir: selected,
        });
        setStatusMessage(`Pasta de saída definida para: ${selected}`);
      }
    } catch (err: any) {
      setErrorMessage(err?.message || 'Não foi possível abrir o seletor de pasta.');
    }
  }

  async function handleOpenOutputFolder(pathToOpen?: string) {
    const target = pathToOpen || project?.export_config.output_dir;
    if (!target) return;
    try {
      await openOutputDirectory(target);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Não foi possível abrir a pasta no sistema.');
    }
  }

  async function handlePreviewVoice() {
    if (!project) return;
    setIsPreviewLoading(true);
    setErrorMessage(null);
    try {
      const res = await previewVoice(previewSampleText, project.tts_config);
      const dataUrl = `data:${res.mime_type};base64,${res.audio_base64}`;
      setPreviewAudioUrl(dataUrl);
      setPreviewEngineLabel(res.used_engine);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Falha ao sintetizar preview.');
    } finally {
      setIsPreviewLoading(false);
    }
  }

  async function handleStartRender() {
    if (!project) return;
    setIsRendering(true);
    setRenderResult(null);
    setErrorMessage(null);
    setStatusMessage('Sintetizando capítulos e empacotando audiobook...');
    try {
      const artifact = await startAudiobookRender();
      setRenderResult(artifact);
      setStatusMessage(`Audiobook gerado em: ${artifact.primary_output_path}`);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Falha ao renderizar audiobook.');
    } finally {
      setIsRendering(false);
    }
  }

  const selectedChapter = project?.chapters.find((c) => c.id === selectedChapterId) || null;

  function formatDuration(secs: number): string {
    const mins = Math.floor(secs / 60);
    const remSecs = Math.round(secs % 60);
    if (mins === 0) return `${remSecs}s`;
    return `${mins}m ${remSecs}s`;
  }

  return (
    <div className="min-h-screen flex flex-col bg-slate-950 text-slate-100">
      {/* Top Navbar */}
      <header className="border-b border-slate-800 bg-slate-900/80 backdrop-blur px-6 py-3.5 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-indigo-600 flex items-center justify-center shadow-lg shadow-indigo-600/30">
            <Headphones className="w-5 h-5 text-white" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="font-bold text-lg tracking-tight">Open Audiobook Creator</h1>
              <span className="text-[11px] font-semibold uppercase tracking-wider px-2 py-0.5 rounded-full bg-indigo-500/20 text-indigo-300 border border-indigo-500/30">
                Rust + Local TTS
              </span>
            </div>
            <p className="text-xs text-slate-400">
              Conversor Open Source Multiplataforma (Windows • Linux • macOS) — PDF, EPUB & TXT para M4B / MP3
            </p>
          </div>
        </div>

        {/* Navigation Pills */}
        <nav className="flex items-center gap-1.5 bg-slate-950/90 p-1.5 rounded-xl border border-slate-800">
          <button
            onClick={() => setActiveTab('import')}
            className={`flex items-center gap-2 px-3.5 py-2 rounded-lg text-xs font-medium transition ${
              activeTab === 'import'
                ? 'bg-indigo-600 text-white shadow'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'
            }`}
          >
            <BookOpen className="w-4 h-4" />
            1. Livro & Limpeza
          </button>
          <button
            onClick={() => setActiveTab('chapters')}
            className={`flex items-center gap-2 px-3.5 py-2 rounded-lg text-xs font-medium transition ${
              activeTab === 'chapters'
                ? 'bg-indigo-600 text-white shadow'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'
            }`}
          >
            <Layers className="w-4 h-4" />
            2. Capítulos & Editor ({project?.chapters.length ?? 0})
          </button>
          <button
            onClick={() => setActiveTab('voices')}
            className={`flex items-center gap-2 px-3.5 py-2 rounded-lg text-xs font-medium transition ${
              activeTab === 'voices'
                ? 'bg-indigo-600 text-white shadow'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'
            }`}
          >
            <Mic className="w-4 h-4" />
            3. Vozes TTS Locais
          </button>
          <button
            onClick={() => setActiveTab('render')}
            className={`flex items-center gap-2 px-3.5 py-2 rounded-lg text-xs font-medium transition ${
              activeTab === 'render'
                ? 'bg-indigo-600 text-white shadow'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'
            }`}
          >
            <Download className="w-4 h-4" />
            4. Exportar M4B / MP3
          </button>
        </nav>
      </header>

      {/* Banner notifications */}
      {(statusMessage || errorMessage) && (
        <div
          className={`px-6 py-2.5 text-xs flex items-center justify-between border-b ${
            errorMessage
              ? 'bg-rose-950/70 border-rose-800 text-rose-200'
              : 'bg-indigo-950/60 border-indigo-800/70 text-indigo-200'
          }`}
        >
          <span>{errorMessage || statusMessage}</span>
          <button
            onClick={() => {
              setErrorMessage(null);
              setStatusMessage(null);
            }}
            className="text-slate-400 hover:text-white ml-4"
          >
            Fechar
          </button>
        </div>
      )}

      {/* Main Content Area */}
      <main className="flex-1 p-6 max-w-7xl w-full mx-auto">
        {!project ? (
          <div className="bg-slate-900 border border-slate-800 rounded-2xl p-10 text-center">
            <p className="text-slate-300 mb-4">Carregando estúdio Open Audiobook Creator...</p>
            <button
              onClick={handleLoadSample}
              className="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-sm font-medium"
            >
              Carregar Livro PDF de Demonstração
            </button>
          </div>
        ) : (
          <>
            {/* TAB 1: IMPORT & METADATA */}
            {activeTab === 'import' && (
              <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
                {/* Left Column: File Dropzone & Cleaning Metrics */}
                <div className="lg:col-span-5 space-y-6">
                  <div className="bg-slate-900 border border-slate-800 rounded-2xl p-6">
                    <h2 className="text-base font-semibold flex items-center gap-2 mb-4">
                      <Upload className="w-4 h-4 text-indigo-400" />
                      Importar Livro (PDF, EPUB ou TXT)
                    </h2>

                    <label className="border-2 border-dashed border-slate-700 hover:border-indigo-500 transition rounded-xl p-6 flex flex-col items-center justify-center cursor-pointer bg-slate-950/50 text-center">
                      <FileText className="w-9 h-9 text-indigo-400 mb-2" />
                      <span className="text-sm font-medium text-slate-200">
                        Clique para selecionar um arquivo PDF, EPUB ou TXT
                      </span>
                      <span className="text-xs text-slate-400 mt-1">
                        Extração automática de sumário, capítulos e limpeza de páginas
                      </span>
                      <input
                        type="file"
                        accept=".pdf,.epub,.txt,.md"
                        onChange={handleFileUpload}
                        className="hidden"
                      />
                    </label>

                    <div className="mt-4 flex items-center justify-between gap-2">
                      <button
                        onClick={handleLoadSample}
                        className="w-full py-2.5 px-3 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-200 flex items-center justify-center gap-2 transition"
                      >
                        <Sparkles className="w-4 h-4 text-amber-400" />
                        Recarregar Livro PDF de Exemplo
                      </button>
                    </div>
                  </div>

                  {/* Smart Cleaning Report */}
                  <div className="bg-slate-900 border border-slate-800 rounded-2xl p-6">
                    <h3 className="text-sm font-semibold flex items-center gap-2 mb-4">
                      <Wand2 className="w-4 h-4 text-emerald-400" />
                      Relatório de Higienização Inteligente
                    </h3>
                    <div className="grid grid-cols-2 gap-3 text-xs">
                      <div className="bg-slate-950 p-3 rounded-xl border border-slate-800">
                        <span className="text-slate-400 block">Capítulos Detectados</span>
                        <span className="text-lg font-bold text-white">
                          {project.cleaning_report.chapters_detected}
                        </span>
                      </div>
                      <div className="bg-slate-950 p-3 rounded-xl border border-slate-800">
                        <span className="text-slate-400 block">Páginas Processadas</span>
                        <span className="text-lg font-bold text-white">
                          {project.cleaning_report.total_pages_processed}
                        </span>
                      </div>
                      <div className="bg-slate-950 p-3 rounded-xl border border-slate-800">
                        <span className="text-slate-400 block">Números de Página Removidos</span>
                        <span className="text-lg font-bold text-emerald-400">
                          {project.cleaning_report.page_numbers_removed}
                        </span>
                      </div>
                      <div className="bg-slate-950 p-3 rounded-xl border border-slate-800">
                        <span className="text-slate-400 block">Hifenizações Unidas</span>
                        <span className="text-lg font-bold text-emerald-400">
                          {project.cleaning_report.hyphenations_fixed}
                        </span>
                      </div>
                      <div className="bg-slate-950 p-3 rounded-xl border border-slate-800">
                        <span className="text-slate-400 block">Total de Palavras</span>
                        <span className="text-lg font-bold text-slate-200">
                          {project.cleaning_report.total_words}
                        </span>
                      </div>
                      <div className="bg-slate-950 p-3 rounded-xl border border-slate-800">
                        <span className="text-slate-400 block">Duração Estimada</span>
                        <span className="text-lg font-bold text-indigo-400">
                          {formatDuration(project.cleaning_report.estimated_audio_duration_secs)}
                        </span>
                      </div>
                    </div>

                    {project.cleaning_report.removed_headers_footers.length > 0 && (
                      <div className="mt-4 pt-3 border-t border-slate-800 text-xs">
                        <span className="text-slate-400 block mb-1">
                          Cabeçalhos/Rodapés repetitivos detectados e filtrados:
                        </span>
                        {project.cleaning_report.removed_headers_footers.map((h, i) => (
                          <code
                            key={i}
                            className="block bg-slate-950 text-amber-300 px-2.5 py-1 rounded border border-slate-800 truncate"
                          >
                            "{h}"
                          </code>
                        ))}
                      </div>
                    )}
                  </div>
                </div>

                {/* Right Column: Book Metadata & Cleaner Rules */}
                <div className="lg:col-span-7 space-y-6">
                  <div className="bg-slate-900 border border-slate-800 rounded-2xl p-6">
                    <h2 className="text-base font-semibold mb-4">
                      Metadados do Audiobook (Tags M4B / ID3v2)
                    </h2>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
                      <div>
                        <label className="block text-slate-400 mb-1">Título da Obra</label>
                        <input
                          type="text"
                          value={project.metadata.title}
                          onChange={(e) =>
                            handleMetadataChange({ ...project.metadata, title: e.target.value })
                          }
                          className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-100 focus:outline-none focus:border-indigo-500"
                        />
                      </div>
                      <div>
                        <label className="block text-slate-400 mb-1">Autor(a)</label>
                        <input
                          type="text"
                          value={project.metadata.author}
                          onChange={(e) =>
                            handleMetadataChange({ ...project.metadata, author: e.target.value })
                          }
                          className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-100 focus:outline-none focus:border-indigo-500"
                        />
                      </div>
                      <div>
                        <label className="block text-slate-400 mb-1">Narrador (Tag Composer)</label>
                        <input
                          type="text"
                          value={project.metadata.narrator}
                          onChange={(e) =>
                            handleMetadataChange({ ...project.metadata, narrator: e.target.value })
                          }
                          className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-100 focus:outline-none focus:border-indigo-500"
                        />
                      </div>
                      <div className="grid grid-cols-2 gap-3">
                        <div>
                          <label className="block text-slate-400 mb-1">Idioma</label>
                          <input
                            type="text"
                            value={project.metadata.language}
                            onChange={(e) =>
                              handleMetadataChange({
                                ...project.metadata,
                                language: e.target.value,
                              })
                            }
                            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-100 focus:outline-none focus:border-indigo-500"
                          />
                        </div>
                        <div>
                          <label className="block text-slate-400 mb-1">Ano</label>
                          <input
                            type="text"
                            value={project.metadata.year ?? ''}
                            onChange={(e) =>
                              handleMetadataChange({ ...project.metadata, year: e.target.value })
                            }
                            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-100 focus:outline-none focus:border-indigo-500"
                          />
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Cleaner Rules Configuration */}
                  <div className="bg-slate-900 border border-slate-800 rounded-2xl p-6">
                    <h2 className="text-base font-semibold flex items-center gap-2 mb-4">
                      <Settings2 className="w-4 h-4 text-indigo-400" />
                      Regras de Limpeza de PDF / Texto
                    </h2>
                    <div className="space-y-3 text-xs">
                      <label className="flex items-center gap-3 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={project.cleaner_options.remove_repeated_headers_footers}
                          onChange={(e) =>
                            handleCleanerChange({
                              ...project.cleaner_options,
                              remove_repeated_headers_footers: e.target.checked,
                            })
                          }
                          className="rounded border-slate-700 text-indigo-600"
                        />
                        <span>
                          Remover cabeçalhos e rodapés repetidos nas margens das páginas
                        </span>
                      </label>

                      <label className="flex items-center gap-3 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={project.cleaner_options.remove_page_numbers}
                          onChange={(e) =>
                            handleCleanerChange({
                              ...project.cleaner_options,
                              remove_page_numbers: e.target.checked,
                            })
                          }
                          className="rounded border-slate-700 text-indigo-600"
                        />
                        <span>
                          Remover numeração de páginas isolada (ex: "Página 12", "14 / 220")
                        </span>
                      </label>

                      <label className="flex items-center gap-3 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={project.cleaner_options.fix_hyphenation}
                          onChange={(e) =>
                            handleCleanerChange({
                              ...project.cleaner_options,
                              fix_hyphenation: e.target.checked,
                            })
                          }
                          className="rounded border-slate-700 text-indigo-600"
                        />
                        <span>
                          Reconstruir palavras hifenizadas na quebra de linha (ex: "naturali-\ndade"
                          → "naturalidade")
                        </span>
                      </label>

                      <div className="pt-2 flex items-center justify-between">
                        <span className="text-slate-400">
                          Tamanho máximo de bloco por sentença TTS (caracteres):
                        </span>
                        <input
                          type="number"
                          min={120}
                          max={500}
                          value={project.cleaner_options.max_chunk_chars}
                          onChange={(e) =>
                            handleCleanerChange({
                              ...project.cleaner_options,
                              max_chunk_chars: Number(e.target.value) || 260,
                            })
                          }
                          className="w-24 bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-right"
                        />
                      </div>
                    </div>

                    <div className="mt-6 flex justify-end">
                      <button
                        onClick={() => setActiveTab('chapters')}
                        className="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-xs font-semibold text-white transition"
                      >
                        Avançar para Revisão de Capítulos →
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* TAB 2: CHAPTERS & INTERACTIVE EDITOR */}
            {activeTab === 'chapters' && (
              <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
                {/* Chapter List Sidebar */}
                <div className="lg:col-span-4 bg-slate-900 border border-slate-800 rounded-2xl p-4 flex flex-col max-h-[75vh]">
                  <div className="flex items-center justify-between mb-3 px-1">
                    <h2 className="text-sm font-semibold">
                      Sumário / Capítulos ({project.chapters.length})
                    </h2>
                    <button
                      onClick={handleAddChapter}
                      className="text-xs px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-indigo-300 flex items-center gap-1"
                    >
                      <Plus className="w-3.5 h-3.5" /> Novo
                    </button>
                  </div>

                  <div className="flex-1 overflow-y-auto space-y-2 pr-1">
                    {project.chapters.map((ch, idx) => {
                      const isSelected = ch.id === selectedChapterId;
                      return (
                        <div
                          key={ch.id}
                          onClick={() => selectChapter(project, ch.id)}
                          className={`p-3 rounded-xl border transition cursor-pointer ${
                            isSelected
                              ? 'bg-indigo-950/60 border-indigo-500/80'
                              : 'bg-slate-950/60 border-slate-800 hover:border-slate-700'
                          }`}
                        >
                          <div className="flex items-center justify-between gap-2">
                            <div className="flex items-center gap-2 min-w-0">
                              <input
                                type="checkbox"
                                checked={ch.enabled}
                                onClick={(e) => e.stopPropagation()}
                                onChange={(e) => handleToggleChapter(ch.id, e.target.checked)}
                                className="rounded border-slate-700 text-indigo-600"
                              />
                              <span className="text-xs font-semibold truncate">{ch.title}</span>
                            </div>
                            <span className="text-[11px] text-slate-400 shrink-0">
                              {ch.chunks.length} blocos
                            </span>
                          </div>

                          <div className="mt-2 flex items-center justify-between text-[11px] text-slate-400">
                            <span>
                              {ch.start_page ? `Pág. ${ch.start_page}` : 'Manual'} •{' '}
                              {ch.cleaned_text.split(/\s+/).filter(Boolean).length} palavras
                            </span>
                            {idx + 1 < project.chapters.length && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleMergeChapter(ch.id);
                                }}
                                title="Mesclar com o próximo capítulo"
                                className="text-indigo-400 hover:text-indigo-300 flex items-center gap-1"
                              >
                                <Merge className="w-3 h-3" /> Mesclar abaixo
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>

                {/* Chapter Text Editor & Sentence Chunks Preview */}
                <div className="lg:col-span-8 bg-slate-900 border border-slate-800 rounded-2xl p-6 flex flex-col">
                  {selectedChapter ? (
                    <>
                      <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
                        <div className="flex-1 min-w-[240px]">
                          <label className="block text-[11px] text-slate-400 mb-1">
                            Título do Capítulo #{selectedChapter.index}
                          </label>
                          <input
                            type="text"
                            value={chapterEditTitle}
                            onChange={(e) => setChapterEditTitle(e.target.value)}
                            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-sm font-medium text-white focus:outline-none focus:border-indigo-500"
                          />
                        </div>
                        <div className="flex items-center gap-2 self-end">
                          <button
                            onClick={() => setShowRawDiff(!showRawDiff)}
                            className="px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-xs text-slate-200"
                          >
                            {showRawDiff ? 'Ocultar Texto Bruto' : 'Comparar com Bruto (PDF)'}
                          </button>
                          <button
                            onClick={handleSaveChapterEdits}
                            className="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-xs font-semibold text-white flex items-center gap-1.5"
                          >
                            <CheckCircle2 className="w-4 h-4" />
                            Salvar Revisão
                          </button>
                        </div>
                      </div>

                      <div
                        className={`grid gap-4 ${
                          showRawDiff ? 'grid-cols-1 md:grid-cols-2' : 'grid-cols-1'
                        }`}
                      >
                        {showRawDiff && (
                          <div>
                            <label className="block text-xs text-amber-400 mb-1.5">
                              Texto Bruto Extraído (Com quebras de linha/hífens originais)
                            </label>
                            <textarea
                              readOnly
                              value={selectedChapter.raw_text}
                              rows={11}
                              className="w-full bg-slate-950/70 border border-slate-800 rounded-xl p-3 text-xs text-slate-400 font-mono leading-relaxed"
                            />
                          </div>
                        )}
                        <div>
                          <label className="block text-xs text-emerald-400 mb-1.5">
                            Texto Limpo Editável (Pronto para Narração TTS)
                          </label>
                          <textarea
                            value={chapterEditText}
                            onChange={(e) => setChapterEditText(e.target.value)}
                            rows={11}
                            className="w-full bg-slate-950 border border-slate-800 rounded-xl p-3 text-xs text-slate-100 leading-relaxed focus:outline-none focus:border-indigo-500"
                          />
                        </div>
                      </div>

                      {/* Sentence Chunks Breakdown */}
                      <div className="mt-5 pt-4 border-t border-slate-800">
                        <h3 className="text-xs font-semibold text-slate-300 mb-2.5">
                          Segmentação em Sentenças para o Motor Neural ({selectedChapter.chunks.length}{' '}
                          trechos)
                        </h3>
                        <div className="max-h-44 overflow-y-auto space-y-1.5 pr-1">
                          {selectedChapter.chunks.map((chunk) => (
                            <div
                              key={chunk.id}
                              className="bg-slate-950 border border-slate-800/90 rounded-lg px-3 py-2 text-xs flex items-center justify-between gap-3"
                            >
                              <span className="text-slate-300">
                                <strong className="text-indigo-400 mr-2">
                                  #{chunk.index + 1}
                                </strong>
                                {chunk.text}
                              </span>
                              <button
                                onClick={() => {
                                  setPreviewSampleText(chunk.text);
                                  setActiveTab('voices');
                                }}
                                className="shrink-0 text-[11px] text-indigo-400 hover:text-indigo-300 px-2 py-1 rounded bg-indigo-950/50 border border-indigo-800/50"
                              >
                                Testar no TTS
                              </button>
                            </div>
                          ))}
                        </div>
                      </div>
                    </>
                  ) : (
                    <p className="text-sm text-slate-400">
                      Selecione um capítulo à esquerda para revisar ou editar o texto.
                    </p>
                  )}
                </div>
              </div>
            )}

            {/* TAB 3: LOCAL TTS ENGINES & VOICE PREVIEW */}
            {activeTab === 'voices' && (
              <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
                {/* Left Column: Engine Selector & Catalog */}
                <div className="lg:col-span-7 space-y-6">
                  <div className="bg-slate-900 border border-slate-800 rounded-2xl p-6">
                    <h2 className="text-base font-semibold flex items-center gap-2 mb-4">
                      <Cpu className="w-4 h-4 text-indigo-400" />
                      Motores de Voz Locais & Open Source
                    </h2>

                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5 mb-5">
                      {(
                        [
                          { id: 'kokoro', label: 'Kokoro-82M', sub: 'GPU / ONNX Realista' },
                          { id: 'piper', label: 'Piper TTS', sub: 'ONNX Rápido CPU' },
                          { id: 'coqui_xtts', label: 'Coqui XTTS-v2', sub: 'Voice Clone' },
                          {
                            id: 'builtin_acoustic',
                            label: 'Acústico Rust',
                            sub: 'Offline Instantâneo',
                          },
                        ] as { id: TtsEngineKind; label: string; sub: string }[]
                      ).map((eng) => {
                        const count =
                          systemInfo?.voices.filter((v) => v.engine === eng.id).length ?? 0;
                        return (
                          <button
                            key={eng.id}
                            onClick={() => {
                              const matchingVoice = systemInfo?.voices.find(
                                (v) => v.engine === eng.id
                              );
                              handleTtsConfigChange({
                                ...project.tts_config,
                                engine: eng.id,
                                voice_id: matchingVoice
                                  ? matchingVoice.id
                                  : project.tts_config.voice_id,
                              });
                            }}
                            className={`p-3 rounded-xl border text-left transition ${
                              project.tts_config.engine === eng.id
                                ? 'bg-indigo-600/20 border-indigo-500 text-white'
                                : 'bg-slate-950 border-slate-800 text-slate-300 hover:border-slate-700'
                            }`}
                          >
                            <div className="flex items-center justify-between">
                              <span className="text-xs font-semibold">{eng.label}</span>
                              <span className="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-300">
                                {count}
                              </span>
                            </div>
                            <div className="text-[10px] text-slate-400 mt-0.5">{eng.sub}</div>
                          </button>
                        );
                      })}
                    </div>

                    {/* Voice Profiles List (Filtered by selected engine) */}
                    <div className="space-y-2.5">
                      {systemInfo?.voices
                        .filter((voice) => voice.engine === project.tts_config.engine)
                        .map((voice) => {
                          const selected = project.tts_config.voice_id === voice.id;
                          return (
                            <div
                              key={voice.id}
                              onClick={() =>
                                handleTtsConfigChange({
                                  ...project.tts_config,
                                  engine: voice.engine,
                                  voice_id: voice.id,
                                })
                              }
                              className={`p-3.5 rounded-xl border cursor-pointer transition flex items-center justify-between gap-4 ${
                                selected
                                  ? 'bg-indigo-950/60 border-indigo-500'
                                  : 'bg-slate-950/70 border-slate-800 hover:border-slate-700'
                              }`}
                            >
                              <div>
                                <div className="flex items-center gap-2 flex-wrap">
                                  <span className="text-xs font-semibold text-white">
                                    {voice.name}
                                  </span>
                                  <span className="text-[10px] px-2 py-0.5 rounded bg-slate-800 text-slate-300">
                                    {voice.language}
                                  </span>
                                  <span className="text-[10px] px-2 py-0.5 rounded bg-slate-800/80 text-slate-300">
                                    {voice.gender}
                                  </span>
                                  <span className="text-[10px] px-2 py-0.5 rounded bg-indigo-950 text-indigo-300 border border-indigo-800/60">
                                    {voice.quality}
                                  </span>
                                </div>
                                <p className="text-xs text-slate-400 mt-1">{voice.description}</p>
                              </div>
                              <div className="text-right shrink-0">
                                <span
                                  className={`text-[11px] font-medium px-2 py-1 rounded-lg ${
                                    voice.installed
                                      ? 'bg-emerald-950 text-emerald-300 border border-emerald-800'
                                      : 'bg-slate-900 text-slate-400 border border-slate-800'
                                  }`}
                                >
                                  {voice.installed ? 'Pronto' : 'Auto-Fallback'}
                                </span>
                              </div>
                            </div>
                          );
                        })}
                    </div>
                  </div>
                </div>

                {/* Right Column: Prosody Controls & Live Audio Preview */}
                <div className="lg:col-span-5 space-y-6">
                  <div className="bg-slate-900 border border-slate-800 rounded-2xl p-6">
                    <h2 className="text-base font-semibold flex items-center gap-2 mb-4">
                      <Volume2 className="w-4 h-4 text-indigo-400" />
                      Tom da Narração & Preview em Tempo Real
                    </h2>

                    <div className="space-y-4 text-xs">
                      {/* Narration Style / Tone Selector */}
                      <div>
                        <label className="block text-slate-400 mb-2">
                          Estilo / Tom de Interpretação
                        </label>
                        <div className="grid grid-cols-2 gap-2">
                          {(
                            [
                              {
                                id: 'storyteller',
                                title: '📖 Contador de Histórias',
                                sub: 'Literário, pausas em reticências & timbre quente',
                              },
                              {
                                id: 'dramatic',
                                title: '🎭 Dramático / Épico',
                                sub: 'Pausas densas, grave & suspense (Prólogos)',
                              },
                              {
                                id: 'professor',
                                title: '🎓 Professor / Didático',
                                sub: 'Dicção clara, articulada & ritmo explicativo',
                              },
                              {
                                id: 'natural',
                                title: '🎙️ Leitura Fluida',
                                sub: 'Cadência neutra e direta',
                              },
                            ] as { id: NarrationStyle; title: string; sub: string }[]
                          ).map((st) => {
                            const active =
                              (project.tts_config.narration_style ?? 'storyteller') === st.id;
                            return (
                              <button
                                key={st.id}
                                type="button"
                                onClick={() =>
                                  handleTtsConfigChange({
                                    ...project.tts_config,
                                    narration_style: st.id,
                                  })
                                }
                                className={`p-2.5 rounded-xl border text-left transition ${
                                  active
                                    ? 'bg-indigo-600/25 border-indigo-500 text-white'
                                    : 'bg-slate-950 border-slate-800 text-slate-300 hover:border-slate-700'
                                }`}
                              >
                                <div className="font-semibold text-xs">{st.title}</div>
                                <div className="text-[10px] text-slate-400 mt-0.5 leading-snug">
                                  {st.sub}
                                </div>
                              </button>
                            );
                          })}
                        </div>
                      </div>

                      <div>
                        <div className="flex justify-between mb-1">
                          <span className="text-slate-400">Velocidade Base da Narração</span>
                          <span className="font-semibold text-indigo-400">
                            {project.tts_config.speed.toFixed(2)}x
                          </span>
                        </div>
                        <input
                          type="range"
                          min="0.6"
                          max="1.8"
                          step="0.05"
                          value={project.tts_config.speed}
                          onChange={(e) =>
                            handleTtsConfigChange({
                              ...project.tts_config,
                              speed: parseFloat(e.target.value),
                            })
                          }
                          className="w-full accent-indigo-500"
                        />
                      </div>

                      <div className="grid grid-cols-2 gap-3">
                        <div>
                          <label className="block text-slate-400 mb-1">
                            Pausa entre Frases (ms)
                          </label>
                          <input
                            type="number"
                            value={project.tts_config.sentence_pause_ms}
                            onChange={(e) =>
                              handleTtsConfigChange({
                                ...project.tts_config,
                                sentence_pause_ms: Number(e.target.value) || 300,
                              })
                            }
                            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5"
                          />
                        </div>
                        <div>
                          <label className="block text-slate-400 mb-1">
                            Pausa entre Parágrafos (ms)
                          </label>
                          <input
                            type="number"
                            value={project.tts_config.paragraph_pause_ms}
                            onChange={(e) =>
                              handleTtsConfigChange({
                                ...project.tts_config,
                                paragraph_pause_ms: Number(e.target.value) || 650,
                              })
                            }
                            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5"
                          />
                        </div>
                      </div>

                      <div>
                        <label className="block text-slate-400 mb-1">
                          Texto de Teste para Amostra de Voz
                        </label>
                        <textarea
                          rows={3}
                          value={previewSampleText}
                          onChange={(e) => setPreviewSampleText(e.target.value)}
                          className="w-full bg-slate-950 border border-slate-800 rounded-xl p-2.5 text-xs text-slate-100"
                        />
                      </div>

                      <button
                        onClick={handlePreviewVoice}
                        disabled={isPreviewLoading}
                        className="w-full py-2.5 px-4 rounded-xl bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-xs font-semibold text-white flex items-center justify-center gap-2 transition"
                      >
                        <Play className="w-4 h-4" />
                        {isPreviewLoading ? 'Sintetizando Amostra...' : 'Ouvir Amostra de Áudio'}
                      </button>

                      {previewAudioUrl && (
                        <div className="bg-slate-950 border border-slate-800 rounded-xl p-3 space-y-2">
                          <div className="text-[11px] text-emerald-400">
                            Sintetizado via: {previewEngineLabel}
                          </div>
                          <audio controls autoPlay src={previewAudioUrl} className="w-full h-9" />
                        </div>
                      )}
                    </div>
                  </div>

                  {/* System Diagnostics Box */}
                  {systemInfo && (
                    <div className="bg-slate-900 border border-slate-800 rounded-2xl p-5 text-xs space-y-2">
                      <div className="font-semibold text-slate-200 mb-2">
                        Diagnóstico do Ambiente Local
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-400">Piper TTS CLI:</span>
                        <span
                          className={
                            systemInfo.diagnostics.piper_installed
                              ? 'text-emerald-400'
                              : 'text-amber-300'
                          }
                        >
                          {systemInfo.diagnostics.piper_installed
                            ? 'Detectado'
                            : 'Usando Fallback Acústico Rust'}
                        </span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-400">Kokoro-82M CLI:</span>
                        <span
                          className={
                            systemInfo.diagnostics.kokoro_installed
                              ? 'text-emerald-400'
                              : 'text-amber-300'
                          }
                        >
                          {systemInfo.diagnostics.kokoro_installed
                            ? 'Detectado'
                            : 'Usando Fallback Acústico Rust'}
                        </span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-400">Masterizador M4B / Capítulos:</span>
                        <span className="text-emerald-400">
                          {systemInfo.diagnostics.ffmpeg_installed
                            ? 'FFmpeg AAC + MP4 Nativo'
                            : 'Container M4B/chpl Nativo Rust'}
                        </span>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* TAB 4: EXPORT & MASTERING */}
            {activeTab === 'render' && (
              <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
                <div className="lg:col-span-5 bg-slate-900 border border-slate-800 rounded-2xl p-6 space-y-5">
                  <h2 className="text-base font-semibold flex items-center gap-2">
                    <Download className="w-4 h-4 text-indigo-400" />
                    Configuração de Exportação
                  </h2>

                  <div className="space-y-3 text-xs">
                    <label className="block text-slate-400">Formato do Audiobook</label>
                    {(
                      [
                        {
                          id: 'm4b',
                          title: 'M4B (Audiobook Nativo com Capítulos & Capa)',
                          desc: 'Ideal para Apple Books, Smart AudioBook Player, Audiobookshelf e VLC.',
                        },
                        {
                          id: 'mp3_single',
                          title: 'MP3 Único com Tags ID3v2',
                          desc: 'Arquivo único compatível com qualquer reprodutor de áudio.',
                        },
                        {
                          id: 'mp3_per_chapter',
                          title: 'MP3 Separado por Capítulo',
                          desc: 'Gera um arquivo MP3 numerado para cada capítulo do livro.',
                        },
                        {
                          id: 'wav',
                          title: 'WAV PCM Sem Perdas (Master)',
                          desc: 'Áudio bruto 16-bit + manifesto FFMETADATA1 de capítulos.',
                        },
                      ] as { id: ExportConfig['format']; title: string; desc: string }[]
                    ).map((fmt) => (
                      <div
                        key={fmt.id}
                        onClick={() =>
                          handleExportConfigChange({
                            ...project.export_config,
                            format: fmt.id,
                          })
                        }
                        className={`p-3.5 rounded-xl border cursor-pointer transition ${
                          project.export_config.format === fmt.id
                            ? 'bg-indigo-950/60 border-indigo-500'
                            : 'bg-slate-950 border-slate-800 hover:border-slate-700'
                        }`}
                      >
                        <div className="font-semibold text-white">{fmt.title}</div>
                        <div className="text-[11px] text-slate-400 mt-0.5">{fmt.desc}</div>
                      </div>
                    ))}
                  </div>

                  <div className="space-y-3 text-xs">
                    <div>
                      <label className="block text-slate-400 mb-1">
                        Pasta de Saída (Onde salvar o Audiobook)
                      </label>
                      <div className="flex items-center gap-2">
                        <input
                          type="text"
                          value={project.export_config.output_dir}
                          onChange={(e) =>
                            handleExportConfigChange({
                              ...project.export_config,
                              output_dir: e.target.value,
                            })
                          }
                          placeholder="Ex: /home/usuario/Audiobooks"
                          className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-100 focus:outline-none focus:border-indigo-500"
                        />
                        <button
                          type="button"
                          onClick={handlePickOutputFolder}
                          className="px-3.5 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white font-semibold flex items-center gap-1.5 shrink-0 transition shadow"
                          title="Selecionar pasta no computador"
                        >
                          <FolderOpen className="w-4 h-4" />
                          Escolher Pasta...
                        </button>
                      </div>
                    </div>

                    <div>
                      <label className="block text-slate-400 mb-1">Bitrate de Áudio (kbps)</label>
                      <input
                        type="number"
                        value={project.export_config.bitrate_kbps}
                        onChange={(e) =>
                          handleExportConfigChange({
                            ...project.export_config,
                            bitrate_kbps: Number(e.target.value) || 128,
                          })
                        }
                        className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2"
                      />
                    </div>
                  </div>

                  <button
                    onClick={handleStartRender}
                    disabled={isRendering}
                    className="w-full py-3 px-4 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-sm font-bold text-white flex items-center justify-center gap-2 shadow-lg shadow-emerald-600/20 transition"
                  >
                    {isRendering ? (
                      <>
                        <RefreshCw className="w-4 h-4 animate-spin" />
                        Sintetizando e Masterizando...
                      </>
                    ) : (
                      <>
                        <Headphones className="w-4 h-4" />
                        Gerar Audiobook Agora
                      </>
                    )}
                  </button>
                </div>

                {/* Right Column: Render Output & Chapter Timestamps */}
                <div className="lg:col-span-7 bg-slate-900 border border-slate-800 rounded-2xl p-6">
                  <h2 className="text-base font-semibold mb-4">
                    Resultado da Masterização & Marcadores de Capítulos
                  </h2>

                  {!renderResult ? (
                    <div className="bg-slate-950 border border-slate-800 rounded-xl p-8 text-center text-xs text-slate-400">
                      Clique em <strong>"Gerar Audiobook Agora"</strong> ao lado para sintetizar os{' '}
                      {project.chapters.filter((c) => c.enabled).length} capítulos habilitados e
                      gerar o arquivo final com índice de capítulos.
                    </div>
                  ) : (
                    <div className="space-y-4 text-xs">
                      <div className="bg-emerald-950/40 border border-emerald-700/60 rounded-xl p-4 space-y-1.5">
                        <div className="text-emerald-300 font-bold text-sm">
                          ✅ Audiobook Exportado com Sucesso!
                        </div>
                        <div>
                          <span className="text-slate-400">Arquivo Gerado: </span>
                          <code className="text-white bg-slate-950 px-2 py-0.5 rounded">
                            {renderResult.primary_output_path}
                          </code>
                        </div>
                        <div>
                          <span className="text-slate-400">Motor Utilizado: </span>
                          <span className="text-slate-200">{renderResult.used_engine}</span>
                        </div>
                        <div>
                          <span className="text-slate-400">Codificador: </span>
                          <span className="text-slate-200">{renderResult.encoder_note}</span>
                        </div>
                        <div>
                          <span className="text-slate-400">Duração Total do Áudio: </span>
                          <span className="text-indigo-300 font-semibold">
                            {renderResult.total_duration_secs.toFixed(1)}s
                          </span>
                        </div>
                        <div className="pt-2">
                          <button
                            type="button"
                            onClick={() => handleOpenOutputFolder(project.export_config.output_dir)}
                            className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-100 font-medium inline-flex items-center gap-1.5 transition"
                          >
                            <FolderOpen className="w-3.5 h-3.5 text-indigo-400" />
                            Abrir Pasta de Saída no Sistema
                          </button>
                        </div>
                      </div>

                      <div>
                        <h3 className="font-semibold text-slate-300 mb-2">
                          Tabela de Capítulos Embutida no M4B ({renderResult.chapter_markers.length}{' '}
                          capítulos)
                        </h3>
                        <div className="space-y-1.5">
                          {renderResult.chapter_markers.map((m) => (
                            <div
                              key={m.index}
                              className="bg-slate-950 border border-slate-800 rounded-lg px-3.5 py-2 flex items-center justify-between"
                            >
                              <span className="font-medium text-slate-200">
                                #{m.index} — {m.title}
                              </span>
                              <span className="font-mono text-indigo-400">
                                {(m.start_ms / 1000).toFixed(1)}s → {(m.end_ms / 1000).toFixed(1)}s
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
          </>
        )}
      </main>
    </div>
  );
}

