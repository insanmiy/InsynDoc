import { Component, lazy, Suspense, useCallback, useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore, type ReactNode } from 'react';
import type { Editor } from '@docx-editor.dev/core/contracts/editor';
import { countDocumentText, desktop, isCapsLockOn, watchExternalLinks, setWindowTitle, setWindowTheme, watchClose, type TextStatistics } from './bridge';
import { DocumentSession, type DocumentAdapter, type UnsavedChoice } from './document-session';
import { readPreferences, storePreferences, type Preferences } from './preferences';
import { Modal } from './Modal';
import { Icon } from './Icon';
import { DownloadMenu } from './DownloadMenu';
import { checkForUpdates, findUpdate, installUpdate, type PendingUpdate, type UpdateCheckResult } from './updater';
import { version as appVersion } from '../package.json';

const EditorWorkspace = lazy(() => import('./EditorWorkspace'));
const Settings = lazy(() => import('./Settings').then((module) => ({ default: module.Settings })));

class EditorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() {
    if (this.state.failed) return <div className="editor-failure" role="alert"><Icon name="document" size={32}/><h2>The editor could not start</h2><p>Close and reopen InsynDoc to try again.</p></div>;
    return this.props.children;
  }
}

function WorkspaceSkeleton() {
  return <div className="workspace-skeleton" aria-label="Loading editor"><div className="skeleton-toolbar"><span/><span/><span/></div><div className="skeleton-stage"><div className="skeleton-page"><span>Opening your workspace…</span></div></div></div>;
}

export function App() {
  const confirmRef = useRef<((choice: UnsavedChoice) => void) | null>(null);
  const [confirmation, setConfirmation] = useState(false);
  const [preferences, setPreferences] = useState(readPreferences);
  const preferencesRef = useRef(preferences);
  preferencesRef.current = preferences;
  const [session] = useState(() => new DocumentSession(desktop, () => new Promise((resolve) => {
    confirmRef.current = resolve;
    setConfirmation(true);
  }), { sanitizeMetadata: () => preferencesRef.current.sanitizeMetadataOnExport }));
  const state = useSyncExternalStore(session.subscribe, session.getSnapshot);
  const [systemDark, setSystemDark] = useState(() => matchMedia('(prefers-color-scheme: dark)').matches);
  const theme = preferences.theme === 'system' ? (systemDark ? 'dark' : 'light') : preferences.theme;
  const browserPathRef = useRef(preferences.browserPath);
  browserPathRef.current = preferences.browserPath;
  const [linkError, setLinkError] = useState<string | null>(null);
  useEffect(() => watchExternalLinks(() => browserPathRef.current, setLinkError), []);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [statisticsOpen, setStatisticsOpen] = useState(false);
  const [selectedTextForStatistics, setSelectedTextForStatistics] = useState('');
  const [selectionStatistics, setSelectionStatistics] = useState<TextStatistics | null>(null);
  const [statisticsPending, setStatisticsPending] = useState(false);
  const [statisticsError, setStatisticsError] = useState<string | null>(null);
  const [goToPageOpen, setGoToPageOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [pageNumber, setPageNumber] = useState('1');
  const [pageError, setPageError] = useState<string | null>(null);
  const editorRef = useRef<Editor | null>(null);
  const [focusMode, setFocusMode] = useState(false);
  const [capsLock, setCapsLock] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [replaceMode, setReplaceMode] = useState(false);
  const [findRequest, setFindRequest] = useState(0);
  const [pageCount, setPageCount] = useState(1);
  const [statistics, setStatistics] = useState<TextStatistics>({ words: 0, characters: 0, charactersWithoutSpaces: 0 });
  const [menuHost, setMenuHost] = useState<HTMLElement | null>(null);
  const [zoomHost, setZoomHost] = useState<HTMLElement | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [titleDraft, setTitleDraft] = useState('');
  const [update, setUpdate] = useState<PendingUpdate | null>(null);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<{ downloaded: number; total?: number } | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);

  const onReady = useCallback((instance: Editor, adapter: DocumentAdapter) => {
    editorRef.current = instance;
    session.attach(adapter);
    void session.openStartup();
    performance.mark('insyndoc-editor-ready');
  }, [session]);

  useEffect(() => {
    performance.mark('insyndoc-shell-ready');
    const media = matchMedia('(prefers-color-scheme: dark)');
    const update = () => setSystemDark(media.matches);
    const syncCapsLock = () => {
      void isCapsLockOn().then((active) => { if (active !== null) setCapsLock(active); });
    };
    const updateCapsLock = (event: KeyboardEvent | MouseEvent) => {
      if (typeof event.getModifierState === 'function') setCapsLock(event.getModifierState('CapsLock'));
      if ('key' in event && event.key === 'CapsLock') syncCapsLock();
    };
    syncCapsLock();
    media.addEventListener('change', update);
    window.addEventListener('focus', syncCapsLock);
    window.addEventListener('keydown', updateCapsLock, true);
    window.addEventListener('keyup', updateCapsLock, true);
    window.addEventListener('pointerdown', updateCapsLock, true);
    window.addEventListener('pointermove', updateCapsLock, true);
    return () => {
      media.removeEventListener('change', update);
      window.removeEventListener('focus', syncCapsLock);
      window.removeEventListener('keydown', updateCapsLock, true);
      window.removeEventListener('keyup', updateCapsLock, true);
      window.removeEventListener('pointerdown', updateCapsLock, true);
      window.removeEventListener('pointermove', updateCapsLock, true);
    };
  }, []);

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    void setWindowTheme(theme).catch(() => undefined);
  }, [theme]);

  useEffect(() => { storePreferences(preferences); }, [preferences]);

  useEffect(() => { if (settingsOpen) setSettingsLoaded(true); }, [settingsOpen]);

  useEffect(() => {
    if (!preferences.updateNotifications) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void findUpdate().then((pending) => { if (!cancelled && pending) setUpdate(pending); });
    }, 3000);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [preferences.updateNotifications]);

  useEffect(() => { void setWindowTitle(state.name, state.dirty).catch(() => undefined); }, [state.name, state.dirty]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void watchClose(() => { void session.close(); }).then((cleanup) => {
      if (disposed) cleanup(); else unlisten = cleanup;
    }).catch(() => undefined);
    return () => { disposed = true; unlisten?.(); };
  }, [session]);

  const updatePreferences = useCallback((next: Preferences) => setPreferences(next), []);
  const showFind = useCallback((replace = false) => { setReplaceMode(replace); setFindOpen(true); setFindRequest((value) => value + 1); }, []);
  const openSettings = useCallback(() => setSettingsOpen(true), []);
  const openStatistics = useCallback(() => {
    setSelectedTextForStatistics(editorRef.current?.query({ type: 'selectedText' }) ?? '');
    setStatisticsOpen(true);
  }, []);
  const closeStatistics = useCallback(() => {
    setStatisticsOpen(false);
    setSelectedTextForStatistics('');
  }, []);
  useEffect(() => {
    setSelectionStatistics(null);
    setStatisticsError(null);
    setStatisticsPending(statisticsOpen);
    if (!statisticsOpen) return;
    let cancelled = false;
    const documentText = editorRef.current?.query({ type: 'paragraphs' }).map((paragraph) => paragraph.text).join('\n') ?? '';
    void Promise.all([
      countDocumentText(documentText),
      selectedTextForStatistics ? countDocumentText(selectedTextForStatistics) : Promise.resolve(null)
    ]).then(([documentCounts, selectedCounts]) => {
      if (cancelled) return;
      setStatistics(documentCounts);
      setSelectionStatistics(selectedCounts);
    }).catch(() => {
      if (!cancelled) setStatisticsError('The text could not be counted. Close word count and try again.');
    }).finally(() => {
      if (!cancelled) setStatisticsPending(false);
    });
    return () => { cancelled = true; };
  }, [statisticsOpen, selectedTextForStatistics]);
  const closeFind = useCallback(() => setFindOpen(false), []);
  const toggleFocus = useCallback(() => setFocusMode((value) => !value), []);
  const openShortcuts = useCallback(() => setShortcutsOpen(true), []);
  const openGoToPage = useCallback(() => {
    setPageNumber(String(editorRef.current?.getCurrentPage('viewport') ?? 1));
    if (editorRef.current) setPageCount(editorRef.current.snapshot().page.total);
    setPageError(null);
    setGoToPageOpen(true);
  }, []);

  useEffect(() => {
    const shortcut = (event: KeyboardEvent): void => {
      if (event.target instanceof Element && event.target.closest('.document-name-input')) return;
      const modalOpen = Boolean(document.querySelector('dialog[open]'));
      if (event.key === 'Escape' && !modalOpen && !document.querySelector('.spelling-popover') && !document.querySelector('[role="menu"]')) {
        if (findOpen) { event.preventDefault(); event.stopImmediatePropagation(); setFindOpen(false); return; }
        if (focusMode) { event.preventDefault(); setFocusMode(false); return; }
      }
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      const key = event.key.toLowerCase();
      const historyShortcut = key === 'z' || key === 'y';
      if (historyShortcut && event.target instanceof Element && event.target.closest('.document-stage, input, textarea, [contenteditable="true"]')) return;
      const replaceShortcut = key === 'i' && event.shiftKey;
      if (!replaceShortcut && !['n', 'o', 's', 'f', 'g', ',', '/', 'z', 'y'].includes(key)) return;
      event.preventDefault(); event.stopImmediatePropagation();
      if (modalOpen) return;
      if (historyShortcut) {
        if (session.getSnapshot().busy) return;
        editorRef.current?.exec({ type: key === 'y' || event.shiftKey ? 'redo' : 'undo' });
        editorRef.current?.focus();
        return;
      }
      if (key === 'n') void session.newDocument();
      if (key === 'o') void session.open();
      if (key === 's') void (event.shiftKey ? session.exportCopy() : session.save());
      if (key === 'f' && event.shiftKey) setFocusMode((value) => !value);
      else if (key === 'f') showFind();
      if (replaceShortcut) showFind(true);
      if (key === ',') setSettingsOpen(true);
      if (key === 'g') openGoToPage();
      if (key === '/') setShortcutsOpen(true);
    };
    window.addEventListener('keydown', shortcut, true);
    return () => window.removeEventListener('keydown', shortcut, true);
  }, [session, showFind, findOpen, focusMode, openGoToPage]);

  function resolveConfirmation(choice: UnsavedChoice): void {
    confirmRef.current?.(choice); confirmRef.current = null; setConfirmation(false);
  }

  const checkUpdates = useCallback(async (): Promise<UpdateCheckResult> => {
    const result = await checkForUpdates();
    if (result.pending) setUpdate(result.pending);
    return result;
  }, []);

  const dismissUpdate = useCallback(() => {
    updatePreferences({ ...preferences, updateNotifications: false });
    setUpdate(null);
  }, [preferences, updatePreferences]);

  async function applyUpdate(): Promise<void> {
    if (!update || updateInstalling) return;
    setUpdateInstalling(true);
    setUpdateError(null);
    setUpdateProgress(null);
    try {
      await installUpdate(update, (downloaded, total) => setUpdateProgress({ downloaded, total }));
    } catch (error: unknown) {
      setUpdateInstalling(false);
      setUpdateError(error instanceof Error ? error.message : String(error));
    }
  }

  const unavailable = !state.ready || state.busy;

  return <div className={`app-shell ${focusMode ? 'focus-mode' : ''}`} data-theme={theme}>
    <header className="app-header">
      <div className="app-logo"><img src="/branding/insyndoc-logo.png" alt="InsynDoc" width="44" height="44"/></div>
      <div className="document-heading">
        <div className="document-title-row"><h1>{renaming ? <input className="document-name-input" aria-label="Document name" autoFocus value={titleDraft} onFocus={(event) => event.currentTarget.select()} onChange={(event) => setTitleDraft(event.target.value)} onBlur={() => { setRenaming(false); void session.rename(titleDraft); }} onKeyDown={(event) => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') { event.preventDefault(); setRenaming(false); void session.rename(titleDraft).then(() => { if (!session.getSnapshot().error) void (event.shiftKey ? session.exportCopy() : session.save()); }); return; } if (event.key === 'Enter') { event.preventDefault(); event.currentTarget.blur(); } if (event.key === 'Escape') { event.preventDefault(); setRenaming(false); } }}/>
          : <button className="document-name-button" disabled={unavailable} title="Rename document" onClick={() => { setTitleDraft(state.name.replace(/\.docx$/i, '')); setRenaming(true); }}><span>{state.name}</span><Icon name="edit" size={14}/></button>}</h1><span className={`document-save-indicator ${state.dirty ? 'has-changes' : ''}`} title={state.dirty ? 'Unsaved changes' : 'Your document stays on this device'}><Icon name={state.dirty ? 'save' : 'check'} size={16}/><span>{state.dirty ? 'Unsaved changes' : state.token ? 'Saved locally' : 'New document'}</span></span></div>
        <nav ref={setMenuHost} className="main-menu-host" aria-label="Document menus" inert={unavailable}>{!state.ready && <div className="menu-placeholder">File<span>Edit</span><span>View</span><span>Insert</span><span>Format</span><span>Tools</span></div>}</nav>
      </div>
      <div className="header-actions">
        <button className="secondary-button import-document" aria-label="Import DOCX" disabled={unavailable} onClick={() => void session.open()} title="Import DOCX (Ctrl+O)"><Icon name="folder" size={16}/><span>Import</span></button>
        <button className="icon-button" aria-label="Settings" title="Settings (Ctrl+,)" onClick={() => setSettingsOpen(true)}><Icon name="settings"/></button>
        <DownloadMenu disabled={unavailable} onDownload={(format) => void session.download(format)}/>
      </div>
    </header>

    {focusMode && <div className="focus-strip"><span><Icon name="focus" size={16}/>Focus mode{capsLock && <span className="caps-lock-warning" role="status" aria-live="polite"><Icon name="keyboard" size={14}/><span>Caps lock on</span></span>}</span><button onClick={() => setFocusMode(false)}>Exit focus mode <kbd>Esc</kbd></button></div>}
    {linkError && <div className="error-banner" role="alert"><Icon name="info" size={18}/><span>{linkError}</span><button onClick={() => setLinkError(null)} aria-label="Dismiss link error"><Icon name="close" size={18}/></button></div>}
    {state.error && <div className="error-banner" role="alert"><Icon name="info" size={18}/><span>{state.error}</span><button onClick={session.clearError} aria-label="Dismiss error"><Icon name="close" size={18}/></button></div>}

    <main className={`editor-workspace view-${state.view}`} aria-label="Document editor" aria-busy={state.busy}>
      <div className="editor-container" inert={state.busy}>
        <EditorBoundary><Suspense fallback={<WorkspaceSkeleton/>}>
          <EditorWorkspace session={session} theme={theme} preferences={preferences} onPreferences={updatePreferences}
            menuHost={menuHost} zoomHost={zoomHost} onReady={onReady} onPageCount={setPageCount} onStatistics={setStatistics}
            onSettings={openSettings} onWordCount={openStatistics} onGoToPage={openGoToPage} onShortcuts={openShortcuts}
            findOpen={findOpen} replaceMode={replaceMode} findRequest={findRequest} onFind={showFind} onCloseFind={closeFind}
            focusMode={focusMode} onFocusMode={toggleFocus} />
        </Suspense></EditorBoundary>
      </div>
      {state.busy && !confirmation && <div className="busy-indicator" role="status">{state.message}…</div>}
      {state.notice && <div className="workspace-notice" role="status"><span>{state.notice}</span><button className="icon-button" onClick={session.clearNotice} aria-label="Dismiss export notification"><Icon name="close" size={16}/></button></div>}
    </main>

    <footer className="status-bar">
      <div className="document-status">{preferences.showWordCount && <><button aria-label="Word count" onClick={openStatistics}>{statistics.words.toLocaleString()} {preferences.wordGoal > 0 ? `/ ${preferences.wordGoal.toLocaleString()} words` : statistics.words === 1 ? 'word' : 'words'}</button><span className="status-separator"/></>}<button aria-label="Go to page" disabled={!state.ready} onClick={openGoToPage}>{Math.max(1, pageCount)} {pageCount === 1 ? 'page' : 'pages'}</button>{capsLock && !focusMode && <><span className="status-separator"/><span className="caps-lock-warning" role="status" aria-live="polite"><Icon name="keyboard" size={14}/><span>Caps lock on</span></span></>}</div>
      <div><div ref={setZoomHost} className="status-zoom" inert={unavailable}/><span className="status-separator"/><button aria-label="Continuous view" aria-pressed={state.view === 'continuous'} title={state.view === 'pages' ? 'Switch to continuous view' : 'Switch to separate pages'} disabled={unavailable} onClick={() => session.setView(state.view === 'pages' ? 'continuous' : 'pages')}><Icon name="page" size={14}/>View: {state.view === 'pages' ? 'Separate pages' : 'Continuous'}</button></div>
    </footer>

    {settingsLoaded && <Suspense fallback={null}><Settings open={settingsOpen} onClose={() => setSettingsOpen(false)} preferences={preferences} onChange={updatePreferences} appVersion={appVersion} onCheckForUpdates={checkUpdates}/></Suspense>}
    <Modal open={goToPageOpen} title="Go to page" onClose={() => setGoToPageOpen(false)} className="page-modal">
      <form noValidate onSubmit={(event) => {
        event.preventDefault();
        const page = Number(pageNumber);
        if (!Number.isInteger(page) || page < 1 || page > pageCount) { setPageError(`Choose a page from 1 to ${pageCount}.`); return; }
        if (editorRef.current?.scrollToPage(page)) setGoToPageOpen(false);
        else setPageError('This page is not ready. Try again.');
      }}>
        <label className="page-input-label" htmlFor="page-number">Page number</label>
        <input id="page-number" className="page-number-input" type="number" min="1" max={Math.max(1, pageCount)} step="1" autoFocus value={pageNumber} onChange={(event) => { setPageNumber(event.target.value); setPageError(null); }}/>
        <p className="modal-description">{pageCount} {pageCount === 1 ? 'page' : 'pages'} in this document.</p>
        {pageError && <p className="form-error" role="alert">{pageError}</p>}
        <footer className="modal-footer"><button type="submit" className="primary-button">Go to page</button></footer>
      </form>
    </Modal>
    <Modal open={shortcutsOpen} title="Keyboard shortcuts" onClose={() => setShortcutsOpen(false)} className="shortcuts-modal">
      <div className="shortcut-list">{[
        ['New document', 'Ctrl+N'], ['Open document', 'Ctrl+O'], ['Save', 'Ctrl+S'], ['Export DOCX', 'Ctrl+Shift+S'],
        ['Find', 'Ctrl+F'], ['Find and replace', 'Ctrl+Shift+I'], ['Go to page', 'Ctrl+G'], ['Focus mode', 'Ctrl+Shift+F'],
        ['Settings', 'Ctrl+,'], ['Keyboard shortcuts', 'Ctrl+/'], ['Undo', 'Ctrl+Z'], ['Redo', 'Ctrl+Y'],
        ['Bold', 'Ctrl+B'], ['Italic', 'Ctrl+I'], ['Underline', 'Ctrl+U']
      ].map(([action, shortcut]) => <div key={action}><span>{action}</span><kbd>{shortcut}</kbd></div>)}</div>
      <footer className="modal-footer"><button className="primary-button" onClick={() => setShortcutsOpen(false)}>Done</button></footer>
    </Modal>
    <Modal open={statisticsOpen} title="Word count" onClose={closeStatistics} className="statistics-modal">
      {statisticsPending && <p className="modal-description" role="status">Counting text...</p>}
      {statisticsError && <p className="form-error" role="alert">{statisticsError}</p>}
      {selectionStatistics && <section aria-label="Selected text statistics">
        <h3 className="statistics-heading">Selected text</h3><dl className="statistics-grid">
          <div><dt>Words</dt><dd>{selectionStatistics.words.toLocaleString()}</dd></div>
          <div><dt>Characters</dt><dd>{selectionStatistics.characters.toLocaleString()}</dd></div>
          <div><dt>Characters without spaces</dt><dd>{selectionStatistics.charactersWithoutSpaces.toLocaleString()}</dd></div>
        </dl>
      </section>}
      <section aria-label="Document statistics" aria-busy={statisticsPending}>
      <h3 className="statistics-heading">Whole document</h3><dl className="statistics-grid">
        <div><dt>Words</dt><dd>{statistics.words.toLocaleString()}</dd></div><div><dt>Pages</dt><dd>{pageCount}</dd></div>
        <div><dt>Characters</dt><dd>{statistics.characters.toLocaleString()}</dd></div><div><dt>Characters without spaces</dt><dd>{statistics.charactersWithoutSpaces.toLocaleString()}</dd></div>
        <div><dt>Estimated reading time</dt><dd>{Math.ceil(statistics.words / 200)} min</dd></div>
      </dl>{preferences.wordGoal > 0 && <div className="writing-goal"><div><span>Writing goal</span><strong>{Math.min(100, Math.floor(statistics.words / preferences.wordGoal * 100))}%</strong></div><progress aria-label="Writing goal progress" value={Math.min(statistics.words, preferences.wordGoal)} max={preferences.wordGoal}/><p>{statistics.words >= preferences.wordGoal ? 'Goal reached.' : `${(preferences.wordGoal - statistics.words).toLocaleString()} words to go.`}</p></div>}
      </section><footer className="modal-footer"><button className="primary-button" onClick={closeStatistics}>Done</button></footer>
    </Modal>
    <Modal open={update !== null} title="Update available" onClose={() => { if (!updateInstalling) setUpdate(null); }} className="update-modal">
      <p className="modal-description">InsynDoc {update?.version} is available.</p>
      {update?.notes && <p className="update-notes">{update.notes}</p>}
      {updateInstalling && <p className="modal-description" role="status">{updateProgress?.total ? `Downloading update... ${Math.min(100, Math.round(updateProgress.downloaded / updateProgress.total * 100))}%` : 'Downloading update...'}</p>}
      {updateError && <p className="form-error" role="alert">{updateError}</p>}
      <footer className="modal-footer">
        {updateInstalling
          ? <button className="secondary-button" disabled>Installing...</button>
          : <><button className="text-button" onClick={() => setUpdate(null)}>Later</button><button className="text-button" onClick={dismissUpdate}>Don't ask again</button><button className="primary-button" onClick={() => void applyUpdate()}>Update now</button></>}
      </footer>
    </Modal>
    <Modal open={confirmation} title="Save your changes?" onClose={() => resolveConfirmation('cancel')} className="unsaved-dialog">
      <p className="modal-description">Your changes to <strong>{state.name}</strong> have not been saved.</p>
      <div className="dialog-actions"><button className="text-button" onClick={() => resolveConfirmation('discard')}>{state.message.toLowerCase() === 'closing document' ? 'Close without saving' : 'Discard changes'}</button><button className="secondary-button" autoFocus onClick={() => resolveConfirmation('cancel')}>Cancel</button><button className="primary-button" onClick={() => resolveConfirmation('save')}>Save changes</button></div>
    </Modal>
  </div>;
}
