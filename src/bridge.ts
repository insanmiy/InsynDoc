import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { ExportFormat } from './document-export';

export interface SavedFile {
  token: string;
  name: string;
}

export interface DocumentFile extends SavedFile {
  bytes: number[] | Uint8Array;
}

export interface DesktopBridge {
  download?(bytes: Uint8Array, format: ExportFormat, name: string): Promise<string | null>;
  downloadPdf?(name: string): Promise<string | null>;
  openStartup?(): Promise<DocumentFile | null>;
  open(): Promise<DocumentFile | null>;
  save(bytes: Uint8Array, token: string | null, saveAs: boolean, name?: string): Promise<SavedFile | null>;
  rename?(token: string, name: string): Promise<SavedFile>;
  close(): Promise<void>;
  openRecent?(token: string): Promise<DocumentFile>;
  remember?(token: string): Promise<void>;
}

export interface RecentDocument extends SavedFile {
  path: string;
  available: boolean;
}

const MAX_DOCUMENT_BYTES = 25 * 1024 * 1024;
const MAX_METADATA_BYTES = 64 * 1024;

// Binary Packets Contain A Little Endian Metadata Length, UTF8 JSON, Then File Bytes.
function documentPacket(buffer: ArrayBuffer): DocumentFile | null {
  if (!(buffer instanceof ArrayBuffer) || buffer.byteLength < 4) throw new Error('Invalid Document Response.');
  const length = new DataView(buffer).getUint32(0, true);
  if (length > MAX_METADATA_BYTES || length > buffer.byteLength - 4) throw new Error('Invalid Document Metadata.');
  const bytes = new Uint8Array(buffer, 4 + length);
  if (bytes.length > MAX_DOCUMENT_BYTES) throw new Error('The Document Exceeds The 25 MB Limit.');
  const metadata: unknown = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(new Uint8Array(buffer, 4, length)));
  if (metadata === null && bytes.length === 0) return null;
  if (typeof metadata !== 'object' || metadata === null || !('token' in metadata) || typeof metadata.token !== 'string'
    || !('name' in metadata) || typeof metadata.name !== 'string') throw new Error('Invalid Document Metadata.');
  return { token: metadata.token, name: metadata.name, bytes };
}

function savePacket(bytes: Uint8Array, token: string | null, saveAs: boolean, name?: string): Uint8Array {
  if (bytes.length > MAX_DOCUMENT_BYTES) throw new Error('The Document Exceeds The 25 MB Limit.');
  const metadata = new TextEncoder().encode(JSON.stringify({ token, saveAs, name }));
  if (metadata.length > MAX_METADATA_BYTES) throw new Error('Document Metadata Is Too Large.');
  const packet = new Uint8Array(4 + metadata.length + bytes.length);
  new DataView(packet.buffer).setUint32(0, metadata.length, true);
  packet.set(metadata, 4);
  packet.set(bytes, 4 + metadata.length);
  return packet;
}

export async function recentDocuments(): Promise<RecentDocument[]> {
  if (!isTauri()) return [];
  return (await invoke<RecentDocument[] | null>('recent_documents')) ?? [];
}

export async function clearRecentDocuments(): Promise<void> {
  if (isTauri()) await invoke('clear_recent_documents');
}

export async function exportPlainText(text: string, token: string | null): Promise<string | null> {
  if (!isTauri()) throw new Error('Open The Desktop App To Export Plain Text.');
  return invoke<string | null>('export_plain_text', { text, token });
}

export interface TextStatistics {
  words: number;
  characters: number;
  charactersWithoutSpaces: number;
}

export async function countDocumentText(text: string): Promise<TextStatistics> {
  if (isTauri()) return invoke<TextStatistics>('document_statistics', { text });
  const words = new Intl.Segmenter(undefined, { granularity: 'word' }).segment(text);
  return {
    words: Array.from(words).filter((word) => word.isWordLike).length,
    characters: Array.from(text).length,
    charactersWithoutSpaces: Array.from(text).filter((character) => !/\s/u.test(character)).length
  };
}

export const desktop: DesktopBridge = {
  async download(bytes, format, name) {
    if (!isTauri()) throw new Error('Open The Desktop App To Download Documents.');
    if (bytes.length > 100 * 1024 * 1024) throw new Error('The Download Exceeds The 100 MB Limit.');
    const metadata = new TextEncoder().encode(JSON.stringify({ format, name }));
    if (metadata.length > MAX_METADATA_BYTES) throw new Error('Download Metadata Is Too Large.');
    const packet = new Uint8Array(4 + metadata.length + bytes.length);
    new DataView(packet.buffer).setUint32(0, metadata.length, true);
    packet.set(metadata, 4);
    packet.set(bytes, 4 + metadata.length);
    return invoke<string | null>('download_document', packet);
  },
  async downloadPdf(name) {
    if (!isTauri()) throw new Error('Open The Desktop App To Download Documents.');
    return invoke<string | null>('download_pdf', { name });
  },
  async openStartup() {
    if (!isTauri()) return null;
    return documentPacket(await invoke<ArrayBuffer>('open_startup_document'));
  },
  async rename(token, name) { return invoke<SavedFile>('rename_document', { token, name }); },
  async openRecent(token) {
    const file = documentPacket(await invoke<ArrayBuffer>('open_recent_document', { token }));
    if (!file) throw new Error('The Recent Document Is Unavailable.');
    return file;
  },
  async remember(token) {
    if (isTauri()) await invoke('remember_document', { token });
    window.dispatchEvent(new Event('insyndoc:recent-changed'));
  },
  async open() {
    if (!isTauri()) throw new Error('Open The Desktop App To Use Native File Dialogs.');
    return documentPacket(await invoke<ArrayBuffer>('open_document'));
  },
  async save(bytes, token, saveAs, name) {
    if (!isTauri()) throw new Error('Open The Desktop App To Save Documents.');
    return invoke<SavedFile | null>('save_document', savePacket(bytes, token, saveAs, name));
  },
  async close() {
    if (isTauri()) await getCurrentWindow().destroy();
  }
};

export async function watchClose(handler: () => void): Promise<() => void> {
  if (!isTauri()) return () => undefined;
  return getCurrentWindow().onCloseRequested((event) => {
    event.preventDefault();
    handler();
  });
}

export async function setWindowTitle(name: string, dirty: boolean): Promise<void> {
  document.title = `${dirty ? '* ' : ''}${name} | InsynDoc`;
  if (isTauri()) await getCurrentWindow().setTitle(document.title);
}

export async function setWindowTheme(theme: 'light' | 'dark'): Promise<void> {
  if (isTauri()) await getCurrentWindow().setTheme(theme);
}

export async function chooseLinkBrowser(): Promise<string | null> {
  if (!isTauri()) throw new Error('Open The Desktop App To Choose A Browser.');
  return invoke<string | null>('choose_link_browser');
}

export async function openExternalLink(url: string, browserPath: string | null): Promise<void> {
  await invoke('open_external_link', { url, browserPath });
}

export function watchExternalLinks(browserPath: () => string | null, onError: (message: string) => void): () => void {
  if (!isTauri()) return () => undefined;
  const originalOpen = window.open;
  const open: typeof window.open = (url) => {
    if (url) void openExternalLink(String(url), browserPath()).catch((error: unknown) => {
      onError(error instanceof Error ? error.message : String(error));
    });
    return null;
  };
  window.open = open;
  return () => { if (window.open === open) window.open = originalOpen; };
}
