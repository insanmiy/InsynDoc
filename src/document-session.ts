import type { DesktopBridge, DocumentFile } from './bridge';
import type { ExportFormat } from './document-export';

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

export class DocumentSession {
  private state: SessionState = {
    name: 'Untitled.docx', token: null, dirty: false, busy: false,
    ready: false, error: null, view: 'pages', message: 'Opening Editor', notice: null
  };
  private listeners = new Set<() => void>();
  private adapter: DocumentAdapter | null = null;
  private revision = 0;
  private startupRequested = false;

  constructor(
    private readonly bridge: DesktopBridge,
    private readonly confirmUnsaved: () => Promise<UnsavedChoice>
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
    if (!this.state.dirty) this.update({ dirty: true, message: 'Unsaved Changes' });
  };

  setView = (view: ViewMode): void => this.update({ view });
  clearError = (): void => this.update({ error: null });
  clearNotice = (): void => this.update({ notice: null });

  rename = (title: string): Promise<void> => this.run('Renaming Document', async () => {
    const trimmed = title.trim();
    if (!trimmed || /[<>:"/\\|?*\u0000-\u001f]/u.test(trimmed) || /[. ]$/.test(trimmed)) throw new Error('Choose A Name Without Slashes, Reserved Characters, Or A Trailing Dot.');
    const name = /\.docx$/i.test(trimmed) ? trimmed : `${trimmed}.docx`;
    if (name.length > 180 || name.startsWith('.') || /[\u007f-\u009f]/u.test(name) || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(name)) throw new Error('Choose A Shorter Name That Is Not Reserved By Windows.');
    if (name === this.state.name) return;
    if (this.state.token) {
      if (!this.bridge.rename) throw new Error('Document Renaming Is Unavailable.');
      const result = await this.bridge.rename(this.state.token, name);
      this.update(result);
      await this.bridge.remember?.(result.token).catch(() => undefined);
      return;
    }
    this.update({ name, dirty: true });
  });

  private async saveCurrent(saveAs: boolean): Promise<boolean> {
    if (!this.adapter) return false;
    const bytes = await this.adapter.save();
    const revision = this.revision;
    const result = await this.bridge.save(bytes, this.state.token, saveAs, this.state.name);
    if (!result) return false;
    const dirty = revision !== this.revision;
    this.update({ ...result, dirty, message: dirty ? 'Unsaved Changes' : 'Saved To This Computer' });
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
      this.update({ busy: false, message: this.state.dirty ? 'Unsaved Changes' : 'Ready' });
    }
  }

  newDocument = (): Promise<void> => this.run('Creating Document', async () => {
    if (!await this.canReplace()) return;
    await this.adapter!.load();
    this.revision = 0;
    this.update({ name: 'Untitled.docx', token: null, dirty: false });
  });

  open = (): Promise<void> => this.run('Opening Document', async () => {
    if (!await this.canReplace()) return;
    const file = await this.bridge.open();
    if (!file) return;
    await this.loadFile(file);
  });

  openStartup = (): Promise<void> => this.run('Opening Document', async () => {
    if (this.startupRequested || !this.bridge.openStartup) return;
    this.startupRequested = true;
    if (!await this.canReplace()) return;
    const file = await this.bridge.openStartup();
    if (file) await this.loadFile(file);
  });

  openRecent = (token: string): Promise<void> => this.run('Opening Recent Document', async () => {
    if (!this.bridge.openRecent || !await this.canReplace()) return;
    const file = await this.bridge.openRecent(token);
    await this.loadFile(file);
  });

  private async loadFile(file: DocumentFile): Promise<void> {
    const bytes = new Uint8Array(file.bytes);
    await this.adapter!.validate(bytes);
    await this.adapter!.load(bytes);
    this.revision = 0;
    this.update({ name: file.name, token: file.token, dirty: false });
    await this.bridge.remember?.(file.token).catch(() => undefined);
  }

  save = (): Promise<void> => this.run('Saving Document', async () => {
    await this.saveCurrent(false);
  });

  exportCopy = (): Promise<void> => this.run('Exporting DOCX', async () => {
    const bytes = await this.adapter!.save();
    const result = await this.bridge.save(bytes, this.state.token, true, this.state.name);
    if (result) this.update({ notice: `Exported ${result.name}` });
  });

  download = (format: ExportFormat): Promise<void> => this.run('Preparing Download', async () => {
    let filename: string | null;
    if (format === 'pdf') {
      if (!this.bridge.downloadPdf || !this.adapter?.preparePdf) throw new Error('PDF Download Is Unavailable.');
      const cleanup = await this.adapter.preparePdf(this.state.name);
      try { filename = await this.bridge.downloadPdf(this.state.name); }
      finally { cleanup(); }
    } else {
      if (!this.bridge.download) throw new Error('Document Downloads Are Unavailable.');
      let bytes = await this.adapter!.save();
      if (format !== 'docx') {
        const { exportDocument } = await import('./document-export');
        bytes = (await exportDocument(bytes, format, this.state.name)).bytes;
      }
      filename = await this.bridge.download(bytes, format, this.state.name);
    }
    if (filename) this.update({ notice: `Downloaded ${filename}` });
  });

  close = async (): Promise<void> => {
    if (this.state.busy) return;
    this.update({ busy: true, error: null, message: 'Closing Document' });
    try {
      const discardEmpty = this.state.dirty && this.adapter?.isEmpty?.() === true;
      if (!discardEmpty && !await this.canReplace()) return;
      await this.bridge.close();
    } catch (error: unknown) {
      this.update({ error: error instanceof Error ? error.message : String(error) });
    } finally {
      this.update({ busy: false, message: this.state.dirty ? 'Unsaved Changes' : 'Ready' });
    }
  };
}
