import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DocumentSession, type DocumentAdapter, type UnsavedChoice } from '../src/document-session';
import type { DesktopBridge } from '../src/bridge';

describe('Document File Workflows', () => {
  it('Downloads A DOCX Copy Without Changing The Active File Or Dirty State', async () => {
    bridge.download = vi.fn().mockResolvedValue('Report.docx');
    await session.open();
    session.changed();
    await session.download('docx');
    expect(bridge.download).toHaveBeenCalledWith(new Uint8Array([80, 75, 3, 4]), 'docx', 'Opened.docx');
    expect(bridge.save).not.toHaveBeenCalled();
    expect(session.getSnapshot()).toMatchObject({ token: 'opened', name: 'Opened.docx', dirty: true, notice: 'Downloaded Report.docx' });
  });

  it('Cleans Up The PDF Preparation After Cancellation Or Failure', async () => {
    const cleanup = vi.fn();
    adapter.preparePdf = vi.fn().mockResolvedValue(cleanup);
    bridge.downloadPdf = vi.fn().mockResolvedValueOnce(null).mockRejectedValueOnce(new Error('Download Failed'));
    session.changed();
    await session.download('pdf');
    expect(cleanup).toHaveBeenCalledOnce();
    expect(session.getSnapshot()).toMatchObject({ dirty: true, token: null, notice: null });
    await session.download('pdf');
    expect(cleanup).toHaveBeenCalledTimes(2);
    expect(session.getSnapshot()).toMatchObject({ dirty: true, error: 'Download Failed', busy: false });
  });
  it('Closes An Empty Dirty Document Without Asking Or Saving', async () => {
    const confirm = vi.fn().mockResolvedValue('cancel');
    const closingSession = new DocumentSession(bridge, confirm);
    closingSession.attach({ ...adapter, isEmpty: () => true });
    closingSession.changed();
    await closingSession.close();
    expect(confirm).not.toHaveBeenCalled();
    expect(bridge.save).not.toHaveBeenCalled();
    expect(bridge.close).toHaveBeenCalledOnce();
  });

  it('Still Asks Before Closing Nonempty Or Unknown Content', async () => {
    const confirm = vi.fn().mockResolvedValue('cancel');
    const closingSession = new DocumentSession(bridge, confirm);
    closingSession.attach({ ...adapter, isEmpty: () => false });
    closingSession.changed();
    await closingSession.close();
    expect(confirm).toHaveBeenCalledOnce();
    expect(bridge.close).not.toHaveBeenCalled();
  });

  it('Loads The Startup Document Once After The Editor Is Ready', async () => {
    bridge.openStartup = vi.fn().mockResolvedValue({ token: 'startup', name: 'Downloaded Report.docx', bytes: [80, 75, 3, 4] });
    await session.openStartup();
    await session.openStartup();
    expect(bridge.openStartup).toHaveBeenCalledOnce();
    expect(adapter.load).toHaveBeenCalledOnce();
    expect(session.getSnapshot()).toMatchObject({ name: 'Downloaded Report.docx', token: 'startup', dirty: false });
    expect(bridge.open).not.toHaveBeenCalled();
  });

  it('Reports A Startup File Error While Keeping The Blank Editor Usable', async () => {
    bridge.openStartup = vi.fn().mockRejectedValue(new Error('The Document Is Unavailable.'));
    await session.openStartup();
    expect(session.getSnapshot()).toMatchObject({ ready: true, busy: false, token: null, error: 'The Document Is Unavailable.' });
    expect(adapter.load).not.toHaveBeenCalled();
  });
  let bridge: DesktopBridge;
  let adapter: DocumentAdapter;
  let session: DocumentSession;
  let choice: UnsavedChoice;

  beforeEach(() => {
    bridge = {
      open: vi.fn().mockResolvedValue({ token: 'opened', name: 'Opened.docx', bytes: [80, 75, 3, 4] }),
      save: vi.fn().mockResolvedValue({ token: 'saved', name: 'Saved.docx' }),
      close: vi.fn().mockResolvedValue(undefined)
    };
    adapter = { validate: vi.fn().mockResolvedValue(undefined), load: vi.fn().mockResolvedValue(undefined), save: vi.fn().mockResolvedValue(new Uint8Array([80, 75, 3, 4])) };
    choice = 'cancel';
    session = new DocumentSession(bridge, async () => choice);
    session.attach(adapter);
  });

  it('Opens Validated Documents And Resets File Identity On New', async () => {
    await session.open();
    expect(adapter.validate).toHaveBeenCalledBefore(vi.mocked(adapter.load));
    expect(session.getSnapshot()).toMatchObject({ name: 'Opened.docx', token: 'opened', dirty: false });
    await session.newDocument();
    expect(session.getSnapshot()).toMatchObject({ name: 'Untitled.docx', token: null, dirty: false });
  });

  it('Uses The Edited Title In The Save Dialog And Renames Saved Files Without Losing Dirty State', async () => {
    await session.rename('My Report');
    expect(session.getSnapshot()).toMatchObject({ name: 'My Report.docx', dirty: true });
    await session.save();
    expect(bridge.save).toHaveBeenCalledWith(expect.any(Uint8Array), null, false, 'My Report.docx');
    session.changed();
    bridge.rename = vi.fn().mockResolvedValue({ token: 'renamed', name: 'Final.docx' });
    await session.rename('Final');
    expect(bridge.rename).toHaveBeenCalledWith('saved', 'Final.docx');
    expect(session.getSnapshot()).toMatchObject({ token: 'renamed', name: 'Final.docx', dirty: true });
    await session.rename('../Bad');
    expect(session.getSnapshot().name).toBe('Final.docx');
    expect(session.getSnapshot().error).toBeTruthy();
  });

  it('Does Not Replace A Document After Cancel Or Failed Validation', async () => {
    session.changed();
    await session.open();
    expect(bridge.open).not.toHaveBeenCalled();
    choice = 'discard';
    vi.mocked(adapter.validate).mockRejectedValue(new Error('Corrupt Document'));
    await session.open();
    expect(adapter.load).not.toHaveBeenCalled();
    expect(session.getSnapshot()).toMatchObject({ dirty: true, error: 'Corrupt Document' });
  });

  it('Keeps Unsaved Changes When Save Is Cancelled Or Fails', async () => {
    session.changed();
    vi.mocked(bridge.save).mockResolvedValue(null);
    await session.save();
    expect(session.getSnapshot().dirty).toBe(true);
    vi.mocked(bridge.save).mockRejectedValue(new Error('Access Denied'));
    await session.save();
    expect(session.getSnapshot()).toMatchObject({ dirty: true, token: null, error: 'Access Denied', busy: false });
  });

  it('Exports A Copy Without Changing The Working Document Or Its Unsaved State', async () => {
    session.changed();
    vi.mocked(bridge.save).mockImplementation(async () => {
      session.changed();
      return { token: 'copy', name: 'Copy.docx' };
    });
    await session.exportCopy();
    expect(bridge.save).toHaveBeenCalledWith(expect.any(Uint8Array), null, true, 'Untitled.docx');
    expect(session.getSnapshot()).toMatchObject({ token: null, name: 'Untitled.docx', dirty: true, notice: 'Exported Copy.docx' });
  });

  it('Saves Directly To The Same Document On Repeated Saves', async () => {
    session.changed();
    await session.save();
    expect(bridge.save).toHaveBeenLastCalledWith(expect.any(Uint8Array), null, false, 'Untitled.docx');
    session.changed();
    await session.save();
    expect(bridge.save).toHaveBeenLastCalledWith(expect.any(Uint8Array), 'saved', false, 'Saved.docx');
    expect(session.getSnapshot().dirty).toBe(false);
  });

  it('Marks Pending Input Included In Saved Bytes As Clean', async () => {
    session.changed();
    vi.mocked(adapter.save).mockImplementation(async () => {
      session.changed();
      return new Uint8Array([80, 75, 3, 4]);
    });
    await session.save();
    expect(session.getSnapshot()).toMatchObject({ dirty: false, token: 'saved' });
  });

  it('Keeps Changes Made While Writing Saved Bytes Dirty', async () => {
    session.changed();
    vi.mocked(bridge.save).mockImplementation(async () => {
      session.changed();
      return { token: 'saved', name: 'Saved.docx' };
    });
    await session.save();
    expect(session.getSnapshot()).toMatchObject({ dirty: true, token: 'saved' });
  });

  it('Keeps The Active File When Export Is Cancelled Or Fails', async () => {
    await session.open();
    session.changed();
    vi.mocked(bridge.save).mockResolvedValue(null);
    await session.exportCopy();
    expect(session.getSnapshot()).toMatchObject({ token: 'opened', name: 'Opened.docx', dirty: true, notice: null });
    vi.mocked(bridge.save).mockRejectedValue(new Error('Export Failed'));
    await session.exportCopy();
    expect(session.getSnapshot()).toMatchObject({ token: 'opened', dirty: true, error: 'Export Failed', busy: false });
  });

  it('Keeps Unsaved Changes When Closing Is Cancelled', async () => {
    const confirm = vi.fn().mockResolvedValue('cancel');
    const closingSession = new DocumentSession(bridge, confirm);
    closingSession.attach(adapter);
    closingSession.changed();
    await closingSession.close();
    expect(bridge.close).not.toHaveBeenCalled();
    expect(confirm).toHaveBeenCalledOnce();
    expect(bridge.save).not.toHaveBeenCalled();
  });

  it('Closes Without Writing The Document When Unsaved Changes Are Discarded', async () => {
    const confirm = vi.fn(async () => {
      expect(closingSession.getSnapshot().message).toBe('Closing Document');
      return 'discard' as const;
    });
    const closingSession = new DocumentSession(bridge, confirm);
    closingSession.attach(adapter);
    closingSession.changed();
    await closingSession.close();
    expect(confirm).toHaveBeenCalledOnce();
    expect(adapter.save).not.toHaveBeenCalled();
    expect(bridge.save).not.toHaveBeenCalled();
    expect(bridge.close).toHaveBeenCalledOnce();
  });

  it('Does Not Close While Another File Action Is Busy', async () => {
    let complete: (() => void) | undefined;
    vi.mocked(bridge.open).mockImplementation(() => new Promise((resolve) => { complete = () => resolve(null); }));
    const opening = session.open();
    await Promise.resolve();
    await session.close();
    expect(bridge.close).not.toHaveBeenCalled();
    complete!();
    await opening;
  });

  it('Closes Only After A Successful Save Or Discard', async () => {
    session.changed();
    choice = 'save';
    vi.mocked(bridge.save).mockResolvedValueOnce(null).mockRejectedValueOnce(new Error('Save Failed'));
    await session.close();
    await session.close();
    expect(bridge.close).not.toHaveBeenCalled();
    expect(session.getSnapshot()).toMatchObject({ dirty: true, error: 'Save Failed' });
    await session.close();
    expect(bridge.close).toHaveBeenCalledOnce();
    session.changed();
    choice = 'discard';
    await session.close();
    expect(bridge.close).toHaveBeenCalledTimes(2);
  });

  it('Validates Filename Boundaries And Multiple Dots', async () => {
    for (const name of ['.docx', 'CON.extra.docx', '\u{1F600}'.repeat(88) + '.docx']) {
      await session.rename(name);
      expect(session.getSnapshot().error).toBeTruthy();
    }
    for (const name of ['Report.final.docx', '\u{1F600}'.repeat(87) + 'a.docx']) {
      await session.rename(name);
      expect(session.getSnapshot()).toMatchObject({ name, error: null });
    }
  });

  it('Serializes Actions And Changes Views Without Reloading', async () => {
    let complete: (() => void) | undefined;
    vi.mocked(bridge.open).mockImplementation(() => new Promise((resolve) => {
      complete = () => resolve(null);
    }));
    const opening = session.open();
    await Promise.resolve();
    await session.newDocument();
    expect(adapter.load).not.toHaveBeenCalled();
    complete!();
    await opening;
    session.changed();
    session.setView('continuous');
    session.setView('pages');
    expect(session.getSnapshot().dirty).toBe(true);
    expect(adapter.load).not.toHaveBeenCalled();
  });

  it('Allows Closing Before The Editor Finishes Initializing', async () => {
    const startingSession = new DocumentSession(bridge, async () => 'cancel');
    await startingSession.close();
    expect(bridge.close).toHaveBeenCalledOnce();
  });

  it('Protects Unsaved Work And Rejects Invalid Recent Documents', async () => {
    bridge.openRecent = vi.fn().mockResolvedValue({ token: 'recent', name: 'Recent.docx', bytes: [80, 75, 3, 4] });
    bridge.remember = vi.fn().mockResolvedValue(undefined);
    session.changed();
    await session.openRecent('recent');
    expect(bridge.openRecent).not.toHaveBeenCalled();
    choice = 'discard';
    vi.mocked(adapter.validate).mockRejectedValueOnce(new Error('Corrupt Document'));
    await session.openRecent('recent');
    expect(session.getSnapshot()).toMatchObject({ name: 'Untitled.docx', dirty: true });
    expect(adapter.load).not.toHaveBeenCalled();
    await session.openRecent('recent');
    expect(session.getSnapshot()).toMatchObject({ name: 'Recent.docx', dirty: false });
    expect(bridge.remember).toHaveBeenCalledWith('recent');
  });
});
