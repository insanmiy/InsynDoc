import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DocumentSession, type DocumentAdapter, type UnsavedChoice } from '../src/document-session';
import { readDocumentSnapshot, type DesktopBridge } from '../src/bridge';

vi.mock('../src/bridge', async (importOriginal) => ({
  ...await importOriginal<typeof import('../src/bridge')>(),
  readDocumentSnapshot: vi.fn(async () => new Uint8Array([3, 4]))
}));

function createSession(choice: UnsavedChoice, saveSucceeds = true) {
  const confirm = vi.fn(async () => choice);
  const save = vi.fn<DesktopBridge['save']>(async () => saveSucceeds ? { token: 'saved-token', name: 'Saved.docx' } : null);
  const bridge: DesktopBridge = { open: async () => null, save, close: async () => undefined };
  const adapter: DocumentAdapter = {
    validate: vi.fn(async () => undefined),
    load: vi.fn(async () => undefined),
    save: vi.fn(async () => new Uint8Array([1, 2]))
  };
  const session = new DocumentSession(bridge, confirm);
  session.attach(adapter);
  session.changed();
  return { session, adapter, confirm, save };
}

describe('Snapshot Restore', () => {
  beforeEach(() => vi.clearAllMocks());

  it('Keeps Unsaved Edits When Restoration Is Cancelled', async () => {
    const { session, adapter, confirm } = createSession('cancel');
    const restored = await session.restoreSnapshot('older');
    expect(confirm).toHaveBeenCalledOnce();
    expect(adapter.load).not.toHaveBeenCalled();
    expect(restored).toBe(false);
    expect(session.getSnapshot()).toMatchObject({ dirty: true, busy: false });
  });

  it('Does Not Restore When Saving Is Cancelled', async () => {
    const { session, adapter, save } = createSession('save', false);
    const restored = await session.restoreSnapshot('older');
    expect(save).toHaveBeenCalledOnce();
    expect(adapter.load).not.toHaveBeenCalled();
    expect(restored).toBe(false);
  });

  it('Saves Current Edits Before Loading The Original Documents Snapshot', async () => {
    const { session, adapter, save } = createSession('save');
    expect(await session.restoreSnapshot('older')).toBe(true);
    expect(readDocumentSnapshot).toHaveBeenCalledWith(null, 'Untitled.docx', 'older');
    expect(save).toHaveBeenCalledOnce();
    expect(adapter.load).toHaveBeenCalledWith(new Uint8Array([3, 4]));
    expect(save.mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(adapter.load).mock.invocationCallOrder[0]);
    expect(session.getSnapshot()).toMatchObject({ token: 'saved-token', dirty: true });
  });

  it('Restores After Explicitly Discarding Current Edits', async () => {
    const { session, adapter, save } = createSession('discard');
    expect(await session.restoreSnapshot('older')).toBe(true);
    expect(save).not.toHaveBeenCalled();
    expect(adapter.load).toHaveBeenCalledOnce();
  });
});
