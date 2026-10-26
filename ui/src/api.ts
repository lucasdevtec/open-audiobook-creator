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
    __OAC_DESKTOP__?: {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
    };
    __TAURI_INTERNALS__?: {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
    };
  }
}

function getDesktopBridge(): {
  invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
} | null {
  if (typeof window === 'undefined') return null;
  if (window.__OAC_DESKTOP__) return window.__OAC_DESKTOP__;
  if (window.__TAURI_INTERNALS__) return window.__TAURI_INTERNALS__;
  return null;
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  const chunkSize = 0x8000;
  let binary = '';
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const sub = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode.apply(null, Array.from(sub));
  }
  return btoa(binary);
}

export async function fetchSystemInfo(): Promise<SystemInfoResponse> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<SystemInfoResponse>('get_system_info');
  }
  const res = await fetch(`${API_BASE}/api/system`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function fetchProject(): Promise<BookProject> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('get_project');
  }
  const res = await fetch(`${API_BASE}/api/project`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function loadSampleProject(): Promise<BookProject> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('load_sample_project');
  }
  const res = await fetch(`${API_BASE}/api/project/sample`, { method: 'POST' });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function uploadBookFile(file: File): Promise<BookProject> {
  const desktop = getDesktopBridge();
  if (desktop) {
    const buffer = await file.arrayBuffer();
    const data_base64 = arrayBufferToBase64(buffer);
    return desktop.invoke<BookProject>('upload_book_file', {
      filename: file.name,
      data_base64,
    });
  }
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('import_project_from_path', { path });
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('update_metadata', { metadata });
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('update_cleaner_options', { options });
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('update_chapter', {
      chapter_id: params.chapter_id,
      title: params.title ?? null,
      cleaned_text: params.cleaned_text ?? null,
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('merge_chapter', {
      chapter_id,
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('add_custom_chapter', { title, text });
  }
  const res = await fetch(`${API_BASE}/api/project/chapter/add`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, text }),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function saveTtsConfig(tts_config: TtsConfig): Promise<BookProject> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('update_tts_config', {
      tts_config,
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<BookProject>('update_export_config', {
      export_config,
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

export async function pickOutputDirectory(currentDir?: string): Promise<string | null> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<string | null>('pick_output_dir', {
      current_dir: currentDir ?? null,
    });
  }
  const res = await fetch(`${API_BASE}/api/project/pick-output-dir`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ current_dir: currentDir ?? null }),
  });
  if (!res.ok) throw new Error(await res.text());
  const data = (await res.json()) as { path: string | null };
  return data.path ?? null;
}

export async function openOutputDirectory(path: string): Promise<void> {
  const desktop = getDesktopBridge();
  if (desktop) {
    await desktop.invoke('open_output_dir', { path });
    return;
  }
  await fetch(`${API_BASE}/api/project/open-output-dir`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path }),
  });
}

export async function previewVoice(
  text: string,
  tts_config: TtsConfig
): Promise<VoicePreviewResponse> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<VoicePreviewResponse>('preview_voice', {
      text,
      tts_config,
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
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<RenderArtifact>('start_render');
  }
  const res = await fetch(`${API_BASE}/api/render/start`, { method: 'POST' });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export interface WizardProgress {
  percent: number;
  step: string;
  finished: boolean;
  error: string | null;
}

export async function runSetupWizard(): Promise<SystemInfoResponse> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<SystemInfoResponse>('run_setup_wizard');
  }
  const res = await fetch(`${API_BASE}/api/system/wizard`, { method: 'POST' });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function fetchWizardProgress(): Promise<WizardProgress | null> {
  const desktop = getDesktopBridge();
  if (desktop) {
    return desktop.invoke<WizardProgress | null>('get_wizard_progress');
  }
  const res = await fetch(`${API_BASE}/api/system/wizard/progress`);
  if (!res.ok) return null;
  return res.json();
}
