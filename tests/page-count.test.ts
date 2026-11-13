import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { observePageCount, type PageCountSource } from '../src/page-count';

function createSource() {
  let pages = 1;
  let change = () => undefined as void;
  let selectionChange = (_snapshot: { page: { total: number } }) => undefined as void;
  const stopChange = vi.fn();
  const stopSelection = vi.fn();
  const snapshot = vi.fn(() => ({ page: { total: pages } }));
  const source: PageCountSource = {
    snapshot,
    on(event, listener) {
      if (event === 'change') {
        change = listener as () => void;
        return stopChange;
      }
      selectionChange = listener as typeof selectionChange;
      return stopSelection;
    }
  };
  return { source, snapshot, stopChange, stopSelection,
    edit(total: number) { pages = total; change(); },
    select(total: number) { pages = total; selectionChange({ page: { total } }); }
  };
}

describe('Deferred Page Counts', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('Reads The Latest Count Once After A Burst Without Reading During Changes', () => {
    const editor = createSource();
    const publish = vi.fn();
    const stop = observePageCount(editor.source, publish);
    editor.edit(10);
    editor.edit(20);
    expect(editor.snapshot).not.toHaveBeenCalled();
    vi.advanceTimersByTime(100);
    expect(editor.snapshot).toHaveBeenCalledTimes(1);
    expect(publish).toHaveBeenLastCalledWith(20);
    stop();
  });

  it('Does Not Postpone Updates Indefinitely During Continuous Editing', () => {
    const editor = createSource();
    const publish = vi.fn();
    const stop = observePageCount(editor.source, publish);
    vi.advanceTimersByTime(90);
    editor.edit(3);
    vi.advanceTimersByTime(10);
    expect(publish).toHaveBeenLastCalledWith(3);
    editor.edit(4);
    vi.advanceTimersByTime(100);
    expect(publish).toHaveBeenLastCalledWith(4);
    stop();
  });

  it('Uses Selection Snapshots And Cancels Redundant Reads', () => {
    const editor = createSource();
    const publish = vi.fn();
    const stop = observePageCount(editor.source, publish);
    editor.edit(8);
    editor.select(8);
    expect(publish).toHaveBeenLastCalledWith(8);
    vi.advanceTimersByTime(100);
    expect(editor.snapshot).not.toHaveBeenCalled();
    stop();
  });

  it('Cancels Pending Updates And Unsubscribes When The Editor Is Replaced', () => {
    const editor = createSource();
    const publish = vi.fn();
    const stop = observePageCount(editor.source, publish);
    editor.edit(5);
    stop();
    vi.runAllTimers();
    expect(editor.snapshot).not.toHaveBeenCalled();
    expect(publish).not.toHaveBeenCalled();
    expect(editor.stopChange).toHaveBeenCalledOnce();
    expect(editor.stopSelection).toHaveBeenCalledOnce();
  });
});
