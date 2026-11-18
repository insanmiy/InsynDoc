import { describe, expect, it, vi } from 'vitest';
import { DocumentSession, type DocumentAdapter, type UnsavedChoice } from '../src/document-session';
import type { DesktopBridge, SavedFile } from '../src/bridge';

async function createSession(choice: UnsavedChoice = 'cancel') {
  const save = vi.fn<DesktopBridge['save']>(async (_bytes, token, saveAs, name) => ({
    token: saveAs ? 'copy-token' : token ?? 'original-token',
    name: saveAs ? 'Copy.docx' : name ?? 'Original.docx'
  }));
  const remember = vi.fn(async () => undefined);
  const close = vi.fn(async () => undefined);
  const bridge: DesktopBridge = {
    open: async () => ({ token: 'original-token', name: 'Original.docx', bytes: new Uint8Array([1, 2]) }),
    save, remember, close
  };
  const adapter: DocumentAdapter = {
    load: async () => undefined,
    validate: async () => undefined,
    save: vi.fn(async () => new Uint8Array([1, 2]))
  };
  const session = new DocumentSession(bridge, async () => choice);
  session.attach(adapter);
  await session.open();
  save.mockClear();
  remember.mockClear();
  return { session, adapter, save, remember, close };
}

describe('Save As', () => {
  it('Makes The New File Active And Uses It For Subsequent Saves', async () => {
    const { session, save, remember } = await createSession();
    session.changed();
    await session.saveAs();
    expect(save).toHaveBeenNthCalledWith(1, new Uint8Array([1, 2]), 'original-token', true, 'Original.docx', true);
    expect(session.getSnapshot()).toMatchObject({ token: 'copy-token', name: 'Copy.docx', dirty: false, busy: false });
    expect(remember).toHaveBeenCalledWith('copy-token');
    session.changed();
    await session.save();
    expect(save).toHaveBeenNthCalledWith(2, new Uint8Array([1, 2]), 'copy-token', false, 'Copy.docx');
  });

  it('Keeps The Original Identity And Unsaved Changes When Cancelled', async () => {
    const { session, save, remember } = await createSession();
    session.changed();
    save.mockResolvedValueOnce(null);
    await session.saveAs();
    expect(session.getSnapshot()).toMatchObject({ token: 'original-token', name: 'Original.docx', dirty: true, busy: false });
    expect(remember).not.toHaveBeenCalled();
  });

  it('Keeps The Original Identity And Reports A Save Failure', async () => {
    const { session, save } = await createSession();
    session.changed();
    save.mockRejectedValueOnce(new Error('The Destination Is Unavailable.'));
    await session.saveAs();
    expect(session.getSnapshot()).toMatchObject({ token: 'original-token', name: 'Original.docx', dirty: true, busy: false, error: 'The Destination Is Unavailable.' });
  });

  it('Preserves Edits Made While The Document Is Being Serialized', async () => {
    const { session, adapter } = await createSession();
    let finishSerialization!: (bytes: Uint8Array) => void;
    adapter.save = () => new Promise((resolve) => { finishSerialization = resolve; });
    const saving = session.saveAs();
    session.changed();
    finishSerialization(new Uint8Array([1, 2]));
    await saving;
    expect(session.getSnapshot()).toMatchObject({ token: 'copy-token', dirty: true });
  });

  it('Preserves Edits Made While The Save Dialog Is Open', async () => {
    const { session, save } = await createSession();
    let finishSave!: (file: SavedFile) => void;
    save.mockImplementationOnce(() => new Promise((resolve) => { finishSave = resolve; }));
    const saving = session.saveAs();
    await Promise.resolve();
    session.changed();
    finishSave({ token: 'copy-token', name: 'Copy.docx' });
    await saving;
    expect(session.getSnapshot()).toMatchObject({ token: 'copy-token', dirty: true });
  });

  it('Keeps Exported Copies Separate From The Active File', async () => {
    const { session, save, remember } = await createSession();
    session.changed();
    await session.exportCopy();
    expect(save).toHaveBeenCalledWith(new Uint8Array([1, 2]), 'original-token', true, 'Original.docx');
    expect(session.getSnapshot()).toMatchObject({ token: 'original-token', name: 'Original.docx', dirty: true });
    expect(remember).not.toHaveBeenCalled();
  });
});

describe('Closing to uninstall', () => {
  it('Saves unsaved work before opening the uninstaller and closing the app', async () => {
    const { session, save, close } = await createSession('save');
    const openUninstaller = vi.fn(async () => undefined);
    session.changed();

    await session.close(openUninstaller);

    expect(save).toHaveBeenCalledBefore(openUninstaller);
    expect(openUninstaller).toHaveBeenCalledBefore(close);
  });

  it('Keeps the app open when the user cancels the save prompt', async () => {
    const { session, close } = await createSession();
    const openUninstaller = vi.fn(async () => undefined);
    session.changed();

    await session.close(openUninstaller);

    expect(openUninstaller).not.toHaveBeenCalled();
    expect(close).not.toHaveBeenCalled();
    expect(session.getSnapshot()).toMatchObject({ dirty: true, busy: false });
  });

  it('Keeps the app open and reports a failure to launch the uninstaller', async () => {
    const { session, close } = await createSession();
    const openUninstaller = vi.fn(async () => { throw new Error('The uninstaller was not found.'); });

    await session.close(openUninstaller);

    expect(close).not.toHaveBeenCalled();
    expect(session.getSnapshot()).toMatchObject({ busy: false, error: 'The uninstaller was not found.' });
  });
});
