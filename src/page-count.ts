interface PageCountSnapshot {
  readonly page: { readonly total: number };
}

export interface PageCountSource {
  snapshot(): PageCountSnapshot;
  on(event: 'change', listener: () => void): () => void;
  on(event: 'selectionChange', listener: (snapshot: PageCountSnapshot) => void): () => void;
}

export function observePageCount(editor: PageCountSource, publish: (pages: number) => void): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const cancelPending = (): void => {
    clearTimeout(timer);
    timer = undefined;
  };
  const schedule = (): void => {
    if (timer !== undefined) return;
    timer = setTimeout(() => {
      timer = undefined;
      publish(editor.snapshot().page.total);
    }, 100);
  };
  const stopChange = editor.on('change', schedule);
  const stopSelection = editor.on('selectionChange', (snapshot) => {
    cancelPending();
    publish(snapshot.page.total);
  });
  schedule();
  return () => {
    cancelPending();
    stopChange();
    stopSelection();
  };
}
