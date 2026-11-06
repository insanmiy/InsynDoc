import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Editor } from '@docx-editor.dev/core/contracts/editor';
import { createEditorAdapter } from '../src/editor-adapter';

const engine = vi.hoisted(() => ({
  capture: vi.fn(), applyUpdate: vi.fn(), finish: vi.fn(),
  snapshot: vi.fn(), recoveryBytes: vi.fn(), recover: vi.fn(),
  createDocxEditor: vi.fn(), createDocumentRefresh: vi.fn()
}));
vi.mock('../src/document-fonts', () => ({ localFonts: {} }));
vi.mock('../src/blank-document', () => ({ createBlankDocument: () => new Uint8Array([4]) }));
vi.mock('../src/image-export', () => ({ synchronizeImageExtents: (bytes: Uint8Array) => bytes }));
vi.mock('@docx-editor.dev/core/editor', () => ({
  createDocxEditor: engine.createDocxEditor,
  createDocumentRefresh: engine.createDocumentRefresh
}));

describe('Editor Adapter Transactional Loading', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    engine.createDocumentRefresh.mockReturnValue(engine);
    engine.capture.mockResolvedValue({ id: 'current', bytes: new ArrayBuffer(0) });
    engine.applyUpdate.mockResolvedValue({ ok: true });
    engine.snapshot.mockReturnValue({ recoveryAvailable: false });
    engine.recoveryBytes.mockReturnValue(null);
  });

  function setup() {
    const editor = {
      load: vi.fn(), save: vi.fn().mockResolvedValue(new Uint8Array([9]).buffer),
      snapshot: vi.fn().mockReturnValue({ isLoading: false, isOpening: false })
    };
    return { editor, adapter: createEditorAdapter(editor as unknown as Editor) };
  }

  it('Recognizes Blank Pages Without Treating Nontext Content As Empty', () => {
    const { editor, adapter } = setup();
    const node = (localName: string, children: readonly object[] = []) => ({ kind: 'generic', namespaceUri: 'http://schemas.openxmlformats.org/wordprocessingml/2006/main', localName, children });
    const parts = vi.fn(() => [{ root: node('document', [node('body', [node('p')])]) }]);
    Object.assign(editor, { surface: { session: { storyParts: parts } } });
    expect(adapter.isEmpty!()).toBe(true);
    for (const content of [node('tbl'), node('drawing'), node('object'), node('fldSimple'), node('r', [node('t', [{ kind: 'textValue', value: 'Unsaved Text' }])])]) {
      parts.mockReturnValue([{ root: node('document', [node('body', [node('p', [content])])]) }]);
      expect(adapter.isEmpty!()).toBe(false);
    }
    parts.mockReturnValue([{ root: node('hdr', [node('p', [node('r', [node('t', [{ kind: 'textValue', value: 'Header Text' }])])])]) }]);
    expect(adapter.isEmpty!()).toBe(false);
    engine.recoveryBytes.mockReturnValue(new ArrayBuffer(1));
    parts.mockReturnValue([{ root: node('document', [node('body', [node('p')])]) }]);
    expect(adapter.isEmpty!()).toBe(false);
  });

  it('Opens Once Through The Engine Transaction Without A Hidden Editor', async () => {
    const { editor, adapter } = setup();
    const bytes = new Uint8Array([1, 2]);
    await adapter.validate(bytes);
    await adapter.load(bytes);
    expect(engine.createDocxEditor).not.toHaveBeenCalled();
    expect(editor.load).not.toHaveBeenCalled();
    expect(engine.applyUpdate).toHaveBeenCalledExactlyOnceWith({
      submission: expect.objectContaining({ id: 'current' }), sequence: 0, bytes, changes: []
    });
    expect(engine.finish).toHaveBeenCalledOnce();
  });

  it('Rejects Malformed And Unsupported Documents Without A Destructive Load', async () => {
    const { editor, adapter } = setup();
    for (const code of ['invalid-document', 'load-failed']) {
      engine.applyUpdate.mockResolvedValueOnce({ ok: false, code, recovered: code === 'load-failed' });
      await adapter.validate(new Uint8Array([1]));
      await expect(adapter.load(new Uint8Array([1]))).rejects.toThrow('Could Not Be Opened');
    }
    expect(editor.load).not.toHaveBeenCalled();
    expect(engine.finish).toHaveBeenCalledTimes(2);
    await expect(adapter.save()).resolves.toEqual(new Uint8Array([9]));
  });

  it('Leaves The Current Document Alone When Capturing Pending Input Fails', async () => {
    const { editor, adapter } = setup();
    engine.capture.mockRejectedValueOnce(new Error('Pending Input Is Invalid'));
    await expect(adapter.load(new Uint8Array([1]))).rejects.toThrow('Pending Input Is Invalid');
    expect(engine.applyUpdate).not.toHaveBeenCalled();
    expect(editor.load).not.toHaveBeenCalled();
  });

  it('Creates New Documents Through The Same Transaction', async () => {
    const { adapter } = setup();
    await adapter.load();
    expect(engine.applyUpdate).toHaveBeenCalledWith(expect.objectContaining({ bytes: new Uint8Array([4]) }));
  });

  it('Keeps Recovery Bytes Saveable When The Engine Cannot Restore Its Surface', async () => {
    const { editor, adapter } = setup();
    const backup = new Uint8Array([8, 7, 6]);
    engine.recoveryBytes.mockReturnValue(backup.buffer);
    await expect(adapter.save()).resolves.toEqual(backup);
    expect(editor.save).not.toHaveBeenCalled();
    engine.snapshot.mockReturnValue({ recoveryAvailable: true });
    engine.recover.mockResolvedValue(false);
    await expect(adapter.load(new Uint8Array([1]))).rejects.toThrow('Save A Copy');
    expect(engine.capture).not.toHaveBeenCalled();
    expect(engine.applyUpdate).not.toHaveBeenCalled();
  });

  it('Finishes The Submission If The Engine Transaction Throws', async () => {
    const { adapter } = setup();
    engine.applyUpdate.mockRejectedValueOnce(new Error('Transaction Failed'));
    await expect(adapter.load(new Uint8Array([1]))).rejects.toThrow('Transaction Failed');
    expect(engine.finish).toHaveBeenCalledOnce();
  });
});
