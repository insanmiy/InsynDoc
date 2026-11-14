import type { DesktopBridge, DocumentFile, SnapshotInfo, SnapshotStatus, TemplateInfo } from './bridge';
import { documentSnapshotStatus, listDocumentSnapshots, listDocumentTemplates, readDocumentSnapshot, readDocumentTemplate, saveDocumentTemplate, writeDocumentSnapshot } from './bridge';
import type { ExportFormat } from './document-export';
import { sanitizeDocxMetadata } from './document-sanitize';

// The Startup Document Request Starts Before The Editor Bundle Loads So File Opens Do Not Wait.
let startupRequest: Promise<DocumentFile | null> | null = null;
let startupResult: DocumentFile | null | undefined;

export function prefetchStartupDocument(bridge: DesktopBridge): void {
  if (startupRequest || !bridge.openStartup) return;
  startupRequest = bridge.openStartup();
  startupRequest.then(
    (file) => { startupResult = file; },
    () => { startupResult = null; }
  );
}

export type UnsavedChoice = 'save' | 'discard' | 'cancel';
export type ViewMode = 'pages' | 'continuous';

export interface DocumentAdapter {
  preparePdf?(title: string): Promise<() => void>;
  isEmpty?(): boolean;
  validate(bytes: Uint8Array): Promise<void>;
  load(bytes?: Uint8Array): Promise<void>;
  save(): Promise<Uint8Array>;
}

export interface SessionState {
  name: string;
  token: string | null;
  dirty: boolean;
  busy: boolean;
  ready: boolean;
  error: string | null;
  view: ViewMode;
  message: string;
  notice: string | null;
}

export interface SessionHooks {
  sanitizeMetadata?: () => boolean;
}

export class DocumentSession {
  private state: SessionState = {
    name: 'Untitled.docx', token: null, dirty: false, busy: false,
    ready: false, error: null, view: 'pages', message: 'Opening editor', notice: null
  };
  private listeners = new Set<() => void>();
  private adapter: DocumentAdapter | null = null;
  private revision = 0;
  private startupRequested = false;
  private adoptedStartupFile: DocumentFile | null = null;
  private closeRequested = false;

  constructor(
    private readonly bridge: DesktopBridge,
    private readonly confirmUnsaved: () => Promise<UnsavedChoice>,
    private readonly hooks: SessionHooks = {}
  ) {}

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  getSnapshot = (): SessionState => this.state;

  private update(patch: Partial<SessionState>): void {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }

  attach(adapter: DocumentAdapter): void {
    this.adapter = adapter;
    this.update({ ready: true, message: 'Ready' });
  }

  changed = (): void => {
    this.revision += 1;
    if (!this.state.dirty) this.update({ dirty: true, message: 'Unsaved changes' });
  };

  setView = (view: ViewMode): void => this.update({ view });
  clearError = (): void => this.update({ error: null });
  clearNotice = (): void => this.update({ notice: null });

  rename = (title: string): Promise<void> => this.run('Renaming document', async () => {
    const trimmed = title.trim();
    if (!trimmed || /[<>:"/\\|?*\u0000-\u001f]/u.test(trimmed) || /[. ]$/.test(trimmed)) throw new Error('Choose a name without slashes, reserved characters, or a trailing dot.');
    const name = /\.docx$/i.test(trimmed) ? trimmed : `${trimmed}.docx`;
    if (name.length > 180 || name.startsWith('.') || /[\u007f-\u009f]/u.test(name) || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(name)) throw new Error('Choose a shorter name that is not reserved by Windows.');
    if (name === this.state.name) return;
    if (this.state.token) {
      if (!this.bridge.rename) throw new Error('Document renaming is unavailable.');
      const result = await this.bridge.rename(this.state.token, name);
      this.update(result);
      await this.bridge.remember?.(result.token).catch(() => undefined);
      return;
    }
    this.update({ name, dirty: true });
  });

  private async saveBytes(): Promise<Uint8Array> {
    const bytes = await this.adapter!.save();
    return this.hooks.sanitizeMetadata?.() ? sanitizeDocxMetadata(bytes) : bytes;
  }

  private async saveCurrent(saveAs: boolean): Promise<boolean> {
    if (!this.adapter) return false;
    const revision = this.revision;
    const bytes = await this.adapter.save();
    const result = saveAs
      ? await this.bridge.save(bytes, this.state.token, true, this.state.name, true)
      : await this.bridge.save(bytes, this.state.token, false, this.state.name);
    if (!result) return false;
    const dirty = revision !== this.revision;
    this.update({ ...result, dirty, message: dirty ? 'Unsaved changes' : 'Saved to this computer' });
    await this.bridge.remember?.(result.token).catch(() => undefined);
    return !dirty;
  }

  private async canReplace(): Promise<boolean> {
    if (!this.state.dirty) return true;
    const choice = await this.confirmUnsaved();
    if (choice === 'cancel') return false;
    if (choice === 'save') return this.saveCurrent(false);
    return true;
  }

  private async run(message: string, action: () => Promise<void>): Promise<void> {
    if (this.state.busy || !this.adapter) return;
    this.update({ busy: true, error: null, notice: null, message });
    try {
      await action();
    } catch (error: unknown) {
      this.update({ error: error instanceof Error ? error.message : String(error) });
    } finally {
      this.update({ busy: false, message: this.state.dirty ? 'Unsaved changes' : 'Ready' });
      if (this.closeRequested) {
        this.closeRequested = false;
        void this.close();
      }
    }
  }

  newDocument = (): Promise<void> => this.run('Creating document', async () => {
    if (!await this.canReplace()) return;
    await this.adapter!.load();
    this.revision = 0;
    this.update({ name: 'Untitled.docx', token: null, dirty: false });
  });

  open = (): Promise<void> => this.run('Opening document', async () => {
    if (!await this.canReplace()) return;
    const file = await this.bridge.open();
    if (!file) return;
    await this.loadFile(file);
  });

  // Returns The Startup File Only After Its Bytes Have Arrived, So The Editor Can Open It Directly.
  startupDocument = (): DocumentFile | null | undefined => startupResult;

  // Marks A Startup File Whose Bytes Were Handed To The Editor At Mount, Skipping A Second Parse.
  adoptStartupDocument = (file: DocumentFile): void => {
    this.adoptedStartupFile = file;
  };

  openStartup = (): Promise<void> => this.run('Opening document', async () => {
    const adopted = this.adoptedStartupFile;
    if (adopted) {
      this.adoptedStartupFile = null;
      this.startupRequested = true;
      this.revision = 0;
      this.update({ name: adopted.name, token: adopted.token || null, dirty: this.state.dirty });
      if (adopted.token) await this.bridge.remember?.(adopted.token).catch(() => undefined);
      return;
    }
    if (this.startupRequested || !this.bridge.openStartup) return;
    this.startupRequested = true;
    if (!await this.canReplace()) return;
    const file = await (startupRequest ?? this.bridge.openStartup());
    if (file) await this.loadFile(file);
  });

  openRecent = (token: string): Promise<void> => this.run('Opening recent document', async () => {
    if (!this.bridge.openRecent || !await this.canReplace()) return;
    const file = await this.bridge.openRecent(token);
    await this.loadFile(file);
  });

  private async loadFile(file: DocumentFile): Promise<void> {
    let bytes: Uint8Array = new Uint8Array(file.bytes);
    let name = file.name;
    let token = file.token || null;
    if (file.importFormat) {
      const { importDocument } = await import('./document-import');
      const imported = await importDocument(file.name, bytes, file.importFormat);
      bytes = imported.bytes;
      name = imported.name;
      token = null;
    }
    await this.adapter!.validate(bytes);
    await this.adapter!.load(bytes);
    this.revision = 0;
    this.update({ name, token, dirty: false });
    if (token) await this.bridge.remember?.(token).catch(() => undefined);
  }

  openTemplate = (name: string, bytes: Uint8Array): Promise<void> => this.run('Opening template', async () => {
    if (!await this.canReplace()) return;
    await this.adapter!.load(bytes);
    this.revision = 0;
    this.update({ name, token: null, dirty: false });
  });

  save = (): Promise<void> => this.run('Saving document', async () => {
    await this.saveCurrent(false);
  });

  saveAs = (): Promise<void> => this.run('Saving document as', async () => {
    await this.saveCurrent(true);
  });

  exportCopy = (): Promise<void> => this.run('Exporting DOCX', async () => {
    const bytes = await this.saveBytes();
    const result = await this.bridge.save(bytes, this.state.token, true, this.state.name);
    if (result) this.update({ notice: `Exported ${result.name}` });
  });

  download = (format: ExportFormat): Promise<void> => this.run('Preparing download', async () => {
    let filename: string | null;
    if (format === 'pdf') {
      if (!this.bridge.downloadPdf || !this.adapter?.preparePdf) throw new Error('PDF download is unavailable.');
      const cleanup = await this.adapter.preparePdf(this.state.name);
      try { filename = await this.bridge.downloadPdf(this.state.name); }
      finally { cleanup(); }
    } else {
      if (!this.bridge.download) throw new Error('Document downloads are unavailable.');
      let bytes = await this.saveBytes();
      if (format !== 'docx') {
        const { exportDocument } = await import('./document-export');
        bytes = (await exportDocument(bytes, format, this.state.name)).bytes;
      }
      filename = await this.bridge.download(bytes, format, this.state.name);
    }
    if (filename) this.update({ notice: `Downloaded ${filename}` });
  });

  captureSnapshot = async (): Promise<void> => {
    if (!this.adapter || this.state.busy) return;
    try {
      const bytes = await this.adapter.save();
      await writeDocumentSnapshot(bytes, this.state.token, this.state.name);
    } catch {
      return;
    }
  };

  listSnapshots = (): Promise<SnapshotInfo[]> => listDocumentSnapshots(this.state.token, this.state.name);

  snapshotStatus = (): Promise<SnapshotStatus | null> => documentSnapshotStatus(this.state.token, this.state.name);

  restoreSnapshot = (id: string): Promise<void> => this.run('Restoring snapshot', async () => {
    const bytes = await readDocumentSnapshot(this.state.token, this.state.name, id);
    await this.adapter!.validate(bytes);
    await this.adapter!.load(bytes);
    this.revision += 1;
    this.update({ dirty: true });
  });

  listTemplates = (): Promise<TemplateInfo[]> => listDocumentTemplates();

  readTemplate = (id: string): Promise<Uint8Array> => readDocumentTemplate(id);

  saveAsTemplate = (name: string): Promise<void> => this.run('Saving template', async () => {
    const bytes = await this.adapter!.save();
    const template = await saveDocumentTemplate(bytes, name);
    this.update({ notice: `Saved template ${template.name}` });
  });

  close = async (): Promise<void> => {
    if (this.state.busy) {
      if (!this.state.dirty) {
        await this.bridge.close();
        return;
      }
      this.closeRequested = true;
      return;
    }
    this.update({ busy: true, error: null, message: 'Closing document' });
    try {
      const discardEmpty = this.state.dirty && this.adapter?.isEmpty?.() === true;
      if (!discardEmpty && !await this.canReplace()) return;
      await this.bridge.close();
    } catch (error: unknown) {
      this.update({ error: error instanceof Error ? error.message : String(error) });
    } finally {
      this.update({ busy: false, message: this.state.dirty ? 'Unsaved changes' : 'Ready' });
    }
  };
}
