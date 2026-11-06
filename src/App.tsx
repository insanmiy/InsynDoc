import { Component, lazy, Suspense, useCallback, useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore, type ReactNode } from 'react';
import type { Editor } from '@docx-editor.dev/core/contracts/editor';
import { desktop, watchExternalLinks, setWindowTitle, setWindowTheme, watchClose, type TextStatistics } from './bridge';
import { DocumentSession, type DocumentAdapter, type UnsavedChoice } from './document-session';
import { readPreferences, storePreferences, type Preferences } from './preferences';
import { Settings } from './Settings';
import { Modal } from './Modal';
import { Icon } from './Icon';
import { DownloadMenu } from './DownloadMenu';

const EditorWorkspace = lazy(() => import('./EditorWorkspace'));

class EditorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() {
    if (this.state.failed) return <div className="editor-failure" role="alert"><Icon name="document" size={32}/><h2>The Editor Could Not Start</h2><p>Close And Reopen InsynDoc To Try Again.</p></div>;
    return this.props.children;
  }
}

function WorkspaceSkeleton() {
  return <div className="workspace-skeleton" aria-label="Loading Editor"><div className="skeleton-toolbar"><span/><span/><span/></div><div className="skeleton-stage"><div className="skeleton-page"><span>Opening Your Workspace…</span></div></div></div>;
}

export function App() {
  const confirmRef = useRef<((choice: UnsavedChoice) => void) | null>(null);
  const [confirmation, setConfirmation] = useState(false);
  const [session] = useState(() => new DocumentSession(desktop, () => new Promise((resolve) => {
    confirmRef.current = resolve;
    setConfirmation(true);
  })));
  const state = useSyncExternalStore(session.subscribe, session.getSnapshot);
  const [preferences, setPreferences] = useState(readPreferences);
  const [systemDark, setSystemDark] = useState(() => matchMedia('(prefers-color-scheme: dark)').matches);
  const theme = preferences.theme === 'system' ? (systemDark ? 'dark' : 'light') : preferences.theme;
  const browserPathRef = useRef(preferences.browserPath);
  browserPathRef.current = preferences.browserPath;
  const [linkError, setLinkError] = useState<string | null>(null);
  useEffect(() => watchExternalLinks(() => browserPathRef.current, setLinkError), []);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [statisticsOpen, setStatisticsOpen] = useState(false);
  const [goToPageOpen, setGoToPageOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [pageNumber, setPageNumber] = useState('1');
  const [pageError, setPageError] = useState<string | null>(null);
  const editorRef = useRef<Editor | null>(null);
  const [focusMode, setFocusMode] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [replaceMode, setReplaceMode] = useState(false);
  const [findRequest, setFindRequest] = useState(0);
  const [pageCount, setPageCount] = useState(1);
  const [statistics, setStatistics] = useState<TextStatistics>({ words: 0, characters: 0, charactersWithoutSpaces: 0 });
  const [menuHost, setMenuHost] = useState<HTMLElement | null>(null);
  const [zoomHost, setZoomHost] = useState<HTMLElement | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [titleDraft, setTitleDraft] = useState('');

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
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    void setWindowTheme(theme).catch(() => undefined);
  }, [theme]);

  useEffect(() => { storePreferences(preferences); }, [preferences]);

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
  const openStatistics = useCallback(() => setStatisticsOpen(true), []);
  const closeFind = useCallback(() => setFindOpen(false), []);
  const toggleFocus = useCallback(() => setFocusMode((value) => !value), []);
  const openShortcuts = useCallback(() => setShortcutsOpen(true), []);
  const openGoToPage = useCallback(() => {
    setPageNumber(String(editorRef.current?.getCurrentPage('viewport') ?? 1));
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

  const unavailable = !state.ready || state.busy;

  return <div className={`app-shell ${focusMode ? 'focus-mode' : ''}`} data-theme={theme}>
    <header className="app-header">
      <div className="app-logo"><img src="/branding/insyndoc-logo.png" alt="InsynDoc" width="44" height="44"/></div>
      <div className="document-heading">
        <div className="document-title-row"><h1>{renaming ? <input className="document-name-input" aria-label="Document Name" autoFocus value={titleDraft} onFocus={(event) => event.currentTarget.select()} onChange={(event) => setTitleDraft(event.target.value)} onBlur={() => { setRenaming(false); void session.rename(titleDraft); }} onKeyDown={(event) => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') { event.preventDefault(); setRenaming(false); void session.rename(titleDraft).then(() => { if (!session.getSnapshot().error) void (event.shiftKey ? session.exportCopy() : session.save()); }); return; } if (event.key === 'Enter') { event.preventDefault(); event.currentTarget.blur(); } if (event.key === 'Escape') { event.preventDefault(); setRenaming(false); } }}/>
          : <button className="document-name-button" disabled={unavailable} title="Rename Document" onClick={() => { setTitleDraft(state.name.replace(/\.docx$/i, '')); setRenaming(true); }}><span>{state.name}</span><Icon name="edit" size={14}/></button>}</h1><span className={`document-save-indicator ${state.dirty ? 'has-changes' : ''}`} title={state.dirty ? 'Unsaved Changes' : 'Your Document Stays On This Device'}><Icon name={state.dirty ? 'save' : 'check'} size={16}/><span>{state.dirty ? 'Unsaved Changes' : state.token ? 'Saved Locally' : 'New Document'}</span></span></div>
        <nav ref={setMenuHost} className="main-menu-host" aria-label="Document Menus" inert={unavailable}>{!state.ready && <div className="menu-placeholder">File<span>Edit</span><span>View</span><span>Insert</span><span>Format</span><span>Tools</span></div>}</nav>
      </div>
      <div className="header-actions">
        <button className="secondary-button import-document" aria-label="Import DOCX" disabled={unavailable} onClick={() => void session.open()} title="Import DOCX (Ctrl+O)"><Icon name="folder" size={16}/><span>Import</span></button>
        <button className="icon-button" aria-label="Settings" title="Settings (Ctrl+,)" onClick={() => setSettingsOpen(true)}><Icon name="settings"/></button>
        <DownloadMenu disabled={unavailable} onDownload={(format) => void session.download(format)}/>
      </div>
    </header>

    {focusMode && <div className="focus-strip"><span><Icon name="focus" size={16}/>Focus Mode</span><button onClick={() => setFocusMode(false)}>Exit Focus Mode <kbd>Esc</kbd></button></div>}
    {linkError && <div className="error-banner" role="alert"><Icon name="info" size={18}/><span>{linkError}</span><button onClick={() => setLinkError(null)} aria-label="Dismiss Link Error"><Icon name="close" size={18}/></button></div>}
    {state.error && <div className="error-banner" role="alert"><Icon name="info" size={18}/><span>{state.error}</span><button onClick={session.clearError} aria-label="Dismiss Error"><Icon name="close" size={18}/></button></div>}

    <main className={`editor-workspace view-${state.view}`} aria-label="Document Editor" aria-busy={state.busy}>
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
      {state.notice && <div className="workspace-notice" role="status"><span>{state.notice}</span><button className="icon-button" onClick={session.clearNotice} aria-label="Dismiss Export Notification"><Icon name="close" size={16}/></button></div>}
    </main>

    <footer className="status-bar">
      <div className="document-status">{preferences.showWordCount && <><button aria-label="Word Count" onClick={() => setStatisticsOpen(true)}>{statistics.words.toLocaleString()} {preferences.wordGoal > 0 ? `/ ${preferences.wordGoal.toLocaleString()} Words` : statistics.words === 1 ? 'Word' : 'Words'}</button><span className="status-separator"/></>}<button aria-label="Go To Page" disabled={!state.ready} onClick={openGoToPage}>{Math.max(1, pageCount)} {pageCount === 1 ? 'Page' : 'Pages'}</button></div>
      <div><div ref={setZoomHost} className="status-zoom" inert={unavailable}/><span className="status-separator"/><button aria-label="Continuous View" aria-pressed={state.view === 'continuous'} title={state.view === 'pages' ? 'Switch To Continuous View' : 'Switch To Separate Pages'} disabled={unavailable} onClick={() => session.setView(state.view === 'pages' ? 'continuous' : 'pages')}><Icon name="page" size={14}/>View: {state.view === 'pages' ? 'Separate Pages' : 'Continuous'}</button></div>
    </footer>

    <Settings open={settingsOpen} onClose={() => setSettingsOpen(false)} preferences={preferences} onChange={updatePreferences}/>
    <Modal open={goToPageOpen} title="Go To Page" onClose={() => setGoToPageOpen(false)} className="page-modal">
      <form noValidate onSubmit={(event) => {
        event.preventDefault();
        const page = Number(pageNumber);
        if (!Number.isInteger(page) || page < 1 || page > pageCount) { setPageError(`Choose A Page From 1 To ${pageCount}.`); return; }
        if (editorRef.current?.scrollToPage(page)) setGoToPageOpen(false);
        else setPageError('This Page Is Not Ready. Try Again.');
      }}>
        <label className="page-input-label" htmlFor="page-number">Page Number</label>
        <input id="page-number" className="page-number-input" type="number" min="1" max={Math.max(1, pageCount)} step="1" autoFocus value={pageNumber} onChange={(event) => { setPageNumber(event.target.value); setPageError(null); }}/>
        <p className="modal-description">{pageCount} {pageCount === 1 ? 'Page' : 'Pages'} In This Document.</p>
        {pageError && <p className="form-error" role="alert">{pageError}</p>}
        <footer className="modal-footer"><button type="submit" className="primary-button">Go To Page</button></footer>
      </form>
    </Modal>
    <Modal open={shortcutsOpen} title="Keyboard Shortcuts" onClose={() => setShortcutsOpen(false)} className="shortcuts-modal">
      <div className="shortcut-list">{[
        ['New Document', 'Ctrl+N'], ['Open Document', 'Ctrl+O'], ['Save', 'Ctrl+S'], ['Export DOCX', 'Ctrl+Shift+S'],
        ['Find', 'Ctrl+F'], ['Find And Replace', 'Ctrl+Shift+I'], ['Go To Page', 'Ctrl+G'], ['Focus Mode', 'Ctrl+Shift+F'],
        ['Settings', 'Ctrl+,'], ['Keyboard Shortcuts', 'Ctrl+/'], ['Undo', 'Ctrl+Z'], ['Redo', 'Ctrl+Y'],
        ['Bold', 'Ctrl+B'], ['Italic', 'Ctrl+I'], ['Underline', 'Ctrl+U']
      ].map(([action, shortcut]) => <div key={action}><span>{action}</span><kbd>{shortcut}</kbd></div>)}</div>
      <footer className="modal-footer"><button className="primary-button" onClick={() => setShortcutsOpen(false)}>Done</button></footer>
    </Modal>
    <Modal open={statisticsOpen} title="Word Count" onClose={() => setStatisticsOpen(false)} className="statistics-modal">
      <p className="modal-description">Your Document At A Glance.</p><dl className="statistics-grid">
        <div><dt>Words</dt><dd>{statistics.words.toLocaleString()}</dd></div><div><dt>Pages</dt><dd>{pageCount}</dd></div>
        <div><dt>Characters</dt><dd>{statistics.characters.toLocaleString()}</dd></div><div><dt>Characters Without Spaces</dt><dd>{statistics.charactersWithoutSpaces.toLocaleString()}</dd></div>
        <div><dt>Estimated Reading Time</dt><dd>{Math.ceil(statistics.words / 200)} Min</dd></div>
      </dl>{preferences.wordGoal > 0 && <div className="writing-goal"><div><span>Writing Goal</span><strong>{Math.min(100, Math.floor(statistics.words / preferences.wordGoal * 100))}%</strong></div><progress aria-label="Writing Goal Progress" value={Math.min(statistics.words, preferences.wordGoal)} max={preferences.wordGoal}/><p>{statistics.words >= preferences.wordGoal ? 'Goal Reached.' : `${(preferences.wordGoal - statistics.words).toLocaleString()} Words To Go.`}</p></div>}<footer className="modal-footer"><button className="primary-button" onClick={() => setStatisticsOpen(false)}>Done</button></footer>
    </Modal>
    <Modal open={confirmation} title="Save Your Changes?" onClose={() => resolveConfirmation('cancel')} className="unsaved-dialog">
      <p className="modal-description">Your Changes To <strong>{state.name}</strong> Have Not Been Saved.</p>
      <div className="dialog-actions"><button className="text-button" onClick={() => resolveConfirmation('discard')}>{state.message === 'Closing Document' ? 'Close Without Saving' : 'Discard Changes'}</button><button className="secondary-button" autoFocus onClick={() => resolveConfirmation('cancel')}>Cancel</button><button className="primary-button" onClick={() => resolveConfirmation('save')}>Save Changes</button></div>
    </Modal>
  </div>;
}
