import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore, type ComponentProps } from 'react';
import { createPortal } from 'react-dom';
import { DocxEditor, useDocxEditor, useDocumentOutline, useDocumentSearch, useHeaderFooterState } from '@docx-editor.dev/react';
import { createT, en, type TranslationKey } from '@docx-editor.dev/i18n';
import { tableChromeIconPaths } from '@docx-editor.dev/core/editor';
import type { Editor } from '@docx-editor.dev/core/contracts/editor';
import { createEditorAdapter, localFonts } from './editor-adapter';
import { clearRecentDocuments, countDocumentText, exportPlainText, recentDocuments, type RecentDocument, type TextStatistics } from './bridge';
import { DocumentSession, type DocumentAdapter } from './document-session';
import type { Preferences } from './preferences';
import { Icon } from './Icon';
import { Modal } from './Modal';
import { WritingTools, type WritingTool } from './WritingTools';
import { ParagraphStylePicker } from './ParagraphStylePicker';
import { replaceDocumentMatches } from './document-replacement';
import { Spelling } from './Spelling';
import { installAutoformat } from './autoformat';
import { TemplateGallery, VersionHistory } from './DocumentPanels';
import { createBlankDocument } from './blank-document';
import { paragraphTextOf } from '@docx-editor.dev/core/store';
import '@docx-editor.dev/core/styles/editor.css';

const editorShortcuts = new Map<string, string>([
  ['history.undo', 'Ctrl+Z'],
  ['history.redo', 'Ctrl+Y'],
  ['text.bold', 'Ctrl+B'],
  ['text.italic', 'Ctrl+I'],
  ['text.underline', 'Ctrl+U'],
  ['alignment.left', 'Ctrl+L'],
  ['alignment.center', 'Ctrl+E'],
  ['alignment.right', 'Ctrl+R'],
  ['alignment.justify', 'Ctrl+J'],
  ['direction.ltr', 'Ctrl+Left Shift'],
  ['direction.rtl', 'Ctrl+Right Shift'],
  ['format.painter', 'Ctrl+Alt+C'],
  ['script.super', 'Ctrl+Shift+='],
  ['script.sub', 'Ctrl+='],
  ['insert.footnote', 'Ctrl+Alt+F'],
  ['insert.endnote', 'Ctrl+Alt+D'],
  ['insert.pageBreak', 'Ctrl+Enter'],
  ['review.paragraphMarks', 'Ctrl+Shift+8']
]);

const english = createT(en);
const translate = (key: string, params?: Record<string, string | number>): string => {
  if (key.startsWith('shortcut.')) return editorShortcuts.get(key.slice('shortcut.'.length)) ?? key;
  return english(key as TranslationKey, params).replace(/\b\p{L}/gu, (letter) => letter.toLocaleUpperCase('en-US'));
};

function MenuAction(props: ComponentProps<typeof DocxEditor.Menu.Row>) {
  return <DocxEditor.Menu.Row {...props} onSelect={() => {
    const menu = document.querySelector('.main-menu-host [role="menu"]');
    menu?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    props.onSelect?.();
  }}/>;
}

interface WorkspaceProps {
  session: DocumentSession;
  theme: 'light' | 'dark';
  preferences: Preferences;
  onPreferences: (preferences: Preferences) => void;
  menuHost: HTMLElement | null;
  zoomHost: HTMLElement | null;
  onReady: (editor: Editor, adapter: DocumentAdapter) => void;
  onPageCount: (pages: number) => void;
  onStatistics: (statistics: TextStatistics) => void;
  onSettings: () => void;
  onWordCount: () => void;
  onGoToPage: () => void;
  onShortcuts: () => void;
  findOpen: boolean;
  replaceMode: boolean;
  findRequest: number;
  onFind: (replace?: boolean) => void;
  onCloseFind: () => void;
  focusMode: boolean;
  onFocusMode: () => void;
}

function Outline({ outline, onClose }: { outline: ReturnType<typeof useDocumentOutline>; onClose: () => void }) {
  return <aside className="document-outline" aria-label="Document Outline">
    <div className="outline-title"><span>Document Outline</span><button className="icon-button" aria-label="Hide Outline" onClick={onClose}><Icon name="close" size={17}/></button></div>
    {outline.isEmpty ? <div className="outline-empty"><span className="outline-empty-lines"><i/><i/><i/></span><p>Your Document, At A Glance</p><span>Add Headings To Navigate Your Document Here.</span></div>
      : <nav aria-label="Headings">{outline.items.map(({ heading, depth }) => <button key={heading.blockId} title={heading.text || 'Untitled Heading'} aria-current={outline.selectedBlockId === heading.blockId ? 'location' : undefined} onClick={() => { outline.goTo(heading.blockId); if (matchMedia('(max-width: 1024px)').matches) onClose(); }} className={outline.selectedBlockId === heading.blockId ? 'selected' : ''} style={{ paddingLeft: 16 + depth * 14 }}>{heading.text || 'Untitled Heading'}</button>)}</nav>}
  </aside>;
}

function FindPanel({ replaceMode, request, onClose }: { replaceMode: boolean; request: number; onClose: () => void }) {
  const editor = useDocxEditor();
  const search = useDocumentSearch();
  const input = useRef<HTMLInputElement>(null);
  const [replacement, setReplacement] = useState('');
  const [showReplace, setShowReplace] = useState(replaceMode);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { setShowReplace(replaceMode); input.current?.focus(); }, [replaceMode, request]);

  function replace(all: boolean): void {
    if (!editor || !search.query || search.isPending || !search.matches.length) return;
    if (all && search.truncated) {
      setError('Narrow Your Search Before Replacing All Matches.');
      return;
    }
    const matches = all ? [...search.matches].reverse() : [search.activeMatch ?? search.matches[0]];
    setError(replaceDocumentMatches(editor, matches, replacement));
    search.find(search.query, { matchCase: search.matchCase, wholeWord: search.wholeWord });
  }

  return <section className="find-panel" aria-label="Find And Replace">
    <div className="find-panel-heading"><strong>Find In Document</strong><button className="icon-button" onClick={onClose} aria-label="Close Find"><Icon name="close" size={18}/></button></div>
    <div className="find-input-row"><Icon name="search" size={18}/><input ref={input} aria-label="Find Text" placeholder="Search Text" value={search.query} onChange={(event) => { search.setQuery(event.target.value); setError(null); }} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); if (event.shiftKey) search.previous(); else search.next(); } }}/><span className="match-count" aria-live="polite">{search.isPending ? '…' : `${search.activeIndex >= 0 ? `${search.activeIndex + 1} Of ` : ''}${search.matches.length}${search.truncated ? '+' : ''}`}</span></div>
    <div className="find-options"><label><input type="checkbox" checked={search.matchCase} onChange={(event) => search.setMatchCase(event.target.checked)}/>Match Case</label><label><input type="checkbox" checked={search.wholeWord} onChange={(event) => search.setWholeWord(event.target.checked)}/>Whole Words</label><button className="icon-button" onClick={search.previous} disabled={!search.matches.length} aria-label="Previous Match"><Icon name="arrowUp" size={18}/></button><button className="icon-button" onClick={search.next} disabled={!search.matches.length} aria-label="Next Match"><Icon name="arrowDown" size={18}/></button></div>
    <button className="replace-toggle" aria-expanded={showReplace} onClick={() => setShowReplace((value) => !value)}><Icon name="chevronDown" size={15}/>Replace</button>
    {showReplace && <div className="replace-controls"><input aria-label="Replace With" placeholder="Replace With" value={replacement} onChange={(event) => setReplacement(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); replace(event.ctrlKey || event.metaKey); } }}/><div><button className="secondary-button" disabled={!search.matches.length || search.isPending} onClick={() => replace(false)}>Replace</button><button className="primary-button" disabled={!search.matches.length || search.isPending} onClick={() => replace(true)}>Replace All</button></div></div>}
    {error && <p role="alert" className="find-error">{error}</p>}
  </section>;
}

function WorkspaceContent(props: WorkspaceProps) {
  const editor = useDocxEditor();
  const furniture = useHeaderFooterState();
  const outline = useDocumentOutline();
  const state = useSyncExternalStore(props.session.subscribe, props.session.getSnapshot);
  const { preferences, onPreferences, session } = props;
  useLayoutEffect(() => {
    if (!editor || !state.ready || state.busy) return;
    return installAutoformat(editor, { smartPunctuation: preferences.smartPunctuation, snippets: preferences.snippetExpansions });
  }, [editor, state.ready, state.busy, state.token, preferences.smartPunctuation, preferences.snippetExpansions]);

  useEffect(() => {
    if (!preferences.autoRecovery || !state.ready) return;
    const timer = setInterval(() => {
      const snapshot = session.getSnapshot();
      if (snapshot.dirty && !snapshot.busy) void session.captureSnapshot();
    }, 120_000);
    return () => clearInterval(timer);
  }, [preferences.autoRecovery, state.ready, session]);

  useEffect(() => {
    if (!state.ready || state.busy) return;
    let disposed = false;
    void session.snapshotStatus().then((status) => {
      if (disposed || !status?.latestCreatedAt) return;
      if (status.fileModifiedAt !== null && status.latestCreatedAt <= status.fileModifiedAt) return;
      setNotice('A Newer Autosaved Version Exists For This Document. Open Version History To Restore It.');
    }).catch(() => undefined);
    return () => { disposed = true; };
  }, [session, state.ready, state.busy, state.token, state.name]);
  const [narrowWindow, setNarrowWindow] = useState(() => matchMedia('(max-width: 1024px)').matches);
  const [outlineDrawerOpen, setOutlineDrawerOpen] = useState(false);
  const outlineVisible = preferences.showOutline && (!narrowWindow || outlineDrawerOpen);
  useEffect(() => {
    const media = matchMedia('(max-width: 1024px)');
    const update = () => { setNarrowWindow(media.matches); setOutlineDrawerOpen(false); };
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);

  useEffect(() => {
    if (!narrowWindow || !outlineDrawerOpen || props.focusMode || props.findOpen) return;
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key !== 'Escape' || event.defaultPrevented || document.querySelector('dialog[open], [role="menu"]')) return;
      event.preventDefault();
      setOutlineDrawerOpen(false);
    };
    window.addEventListener('keydown', closeOnEscape);
    return () => window.removeEventListener('keydown', closeOnEscape);
  }, [narrowWindow, outlineDrawerOpen, props.focusMode, props.findOpen]);

  function toggleOutline(): void {
    if (narrowWindow) {
      if (!preferences.showOutline) onPreferences({ ...preferences, showOutline: true });
      setOutlineDrawerOpen(!outlineVisible);
      return;
    }
    onPreferences({ ...preferences, showOutline: !preferences.showOutline });
  }

  function closeOutline(): void {
    if (narrowWindow) { setOutlineDrawerOpen(false); return; }
    onPreferences({ ...preferences, showOutline: false });
  }

  const [recent, setRecent] = useState<RecentDocument[]>([]);
  const [recentError, setRecentError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [spellingRequest, setSpellingRequest] = useState(0);
  const [menuSearchOpen, setMenuSearchOpen] = useState(false);
  const [menuQuery, setMenuQuery] = useState('');
  const [writingTool, setWritingTool] = useState<WritingTool | null>(null);
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const toolSelection = useRef<{
    surface: NonNullable<NonNullable<ReturnType<typeof useDocxEditor>>['surface']>;
    selection: ReturnType<NonNullable<NonNullable<ReturnType<typeof useDocxEditor>>['surface']>['state']>['selection'];
    scope: ReturnType<Editor['getActiveScope']>;
  } | null>(null);

  function openToolFinder(): void {
    const surface = editor?.surface;
    surface?.flushPendingInput();
    toolSelection.current = surface && editor ? { surface, selection: surface.state().selection, scope: editor.getActiveScope() } : null;
    setMenuQuery('');
    setMenuSearchOpen(true);
  }

  function restoreToolSelection(): void {
    const saved = toolSelection.current;
    if (!editor || !saved || editor.surface !== saved.surface) return;
    editor.setActiveScope(saved.scope);
    saved.surface.setSelection(saved.selection);
  }

  function runMenuCommand(run: () => unknown): void {
    setMenuSearchOpen(false);
    requestAnimationFrame(() => {
      restoreToolSelection();
      run();
    });
  }

  function runFormattingCommand(command: Parameters<Editor['exec']>[0]): void {
    if (!editor) return;
    const result = editor.exec(command);
    if (!result.ok) { setNotice('This Formatting Cannot Be Applied At The Current Selection.'); return; }
    editor.focus();
  }


  const menuCommands = [
    { label: 'New Document', shortcut: 'Ctrl+N', run: () => void session.newDocument() },
    { label: 'Open Document', shortcut: 'Ctrl+O', run: () => void session.open() },
    { label: 'Save Document', shortcut: 'Ctrl+S', run: () => void session.save() },
    { label: 'Export DOCX', shortcut: 'Ctrl+Shift+S', run: () => void session.exportCopy() },
    { label: 'Check Spelling', shortcut: 'Ctrl+Alt+X', run: () => setSpellingRequest((value) => value + 1) },
    { label: 'Find In Document', shortcut: 'Ctrl+F', run: () => props.onFind() },
    { label: 'Find And Replace', shortcut: 'Ctrl+Shift+I', run: () => props.onFind(true) },
    { label: 'Bold', shortcut: 'Ctrl+B', run: () => runFormattingCommand({ type: 'toggleMark', mark: 'bold' }) },
    { label: 'Italic', shortcut: 'Ctrl+I', run: () => runFormattingCommand({ type: 'toggleMark', mark: 'italic' }) },
    { label: 'Underline', shortcut: 'Ctrl+U', run: () => runFormattingCommand({ type: 'toggleMark', mark: 'underline' }) },
    { label: 'Strikethrough', shortcut: '', run: () => runFormattingCommand({ type: 'toggleMark', mark: 'strike' }) },
    { label: 'Superscript', shortcut: editorShortcuts.get('script.super') ?? '', run: () => runFormattingCommand({ type: 'toggleMark', mark: 'superscript' }) },
    { label: 'Subscript', shortcut: editorShortcuts.get('script.sub') ?? '', run: () => runFormattingCommand({ type: 'toggleMark', mark: 'subscript' }) },
    { label: 'Align Left', shortcut: editorShortcuts.get('alignment.left') ?? '', run: () => runFormattingCommand({ type: 'setAlignment', align: 'left' }) },
    { label: 'Align Center', shortcut: editorShortcuts.get('alignment.center') ?? '', run: () => runFormattingCommand({ type: 'setAlignment', align: 'center' }) },
    { label: 'Align Right', shortcut: editorShortcuts.get('alignment.right') ?? '', run: () => runFormattingCommand({ type: 'setAlignment', align: 'right' }) },
    { label: 'Justify', shortcut: editorShortcuts.get('alignment.justify') ?? '', run: () => runFormattingCommand({ type: 'setAlignment', align: 'justify' }) },
    { label: 'Bulleted List', shortcut: '', run: () => runFormattingCommand({ type: 'toggleList', kind: 'bullet' }) },
    { label: 'Numbered List', shortcut: '', run: () => runFormattingCommand({ type: 'toggleList', kind: 'ordered' }) },
    { label: 'Increase Indent', shortcut: 'Ctrl+M', run: () => runFormattingCommand({ type: 'adjustIndent', direction: 'increase' }) },
    { label: 'Decrease Indent', shortcut: 'Ctrl+Shift+M', run: () => runFormattingCommand({ type: 'adjustIndent', direction: 'decrease' }) },
    { label: 'Clear Formatting', shortcut: '', run: () => runFormattingCommand({ type: 'clearFormatting' }) },
    { label: 'Word Count', shortcut: '', run: props.onWordCount },
    { label: 'Insert Date And Time', shortcut: '', run: () => setWritingTool('date') },
    { label: 'Special Characters', shortcut: '', run: () => setWritingTool('symbols') },
    { label: 'Insert Custom Table', shortcut: '', run: () => setWritingTool('table') },
    { label: 'Math Equation', shortcut: '', run: () => setWritingTool('math') },
    { label: 'Edit Header', shortcut: '', run: () => editFurniture('header') },
    { label: 'Edit Footer', shortcut: '', run: () => editFurniture('footer') },
    { label: 'Go To Page', shortcut: 'Ctrl+G', run: props.onGoToPage },
    { label: 'Document Outline', shortcut: '', run: toggleOutline },
    { label: 'Focus Mode', shortcut: 'Ctrl+Shift+F', run: props.onFocusMode },
    { label: 'Settings', shortcut: 'Ctrl+,', run: props.onSettings },
    { label: 'Keyboard Shortcuts', shortcut: 'Ctrl+/', run: props.onShortcuts }
  ].filter((command) => command.label.toLowerCase().includes(menuQuery.trim().toLowerCase()));

  function editFurniture(position: 'header' | 'footer'): void {
    if (!editor) return;
    const result = editor.exec({ type: 'editHeaderFooter', position });
    if (!result.ok) { setNotice('The Header Or Footer Cannot Be Edited At This Location.'); return; }
    requestAnimationFrame(() => editor.focus());
  }

  useEffect(() => {
    const pasteInline = (event: ClipboardEvent): void => {
      if (!editor?.surface || !(event.target instanceof Element) || !event.target.closest('.document-stage') || event.target.closest('.find-panel')) return;
      const clipboard = event.clipboardData;
      const text = clipboard?.getData('text/plain');
      if (!clipboard || !text || clipboard.files.length > 0) return;
      const surface = editor.surface;
      surface.flushPendingInput();
      const scope = editor.getActiveScope();
      if (scope.kind !== 'body' && scope.kind !== 'headerFooter') return;
      const part = surface.session.partFor(scope);
      const selection = surface.state().selection;
      const paragraphs = surface.session.paragraphIdsIn(scope);
      const anchorIndex = paragraphs.indexOf(selection.anchor.paragraphId);
      const headIndex = paragraphs.indexOf(selection.head.paragraphId);
      const end = anchorIndex > headIndex || (anchorIndex === headIndex && selection.anchor.offset > selection.head.offset) ? selection.anchor : selection.head;
      const tailLength = part ? Math.max(0, (paragraphTextOf(part, end.paragraphId)?.length ?? 0) - end.offset) : 0;
      const followingParagraphs = paragraphs.slice(Math.max(anchorIndex, headIndex) + 1);
      const revision = surface.session.packageRevision();
      let html = clipboard.getData('text/html') || undefined;
      let pastedText = text;
      const singleLineText = text.replace(/[\r\n]+$/, '');
      if (html && !/[\r\n]/.test(singleLineText)) {
        const body = new DOMParser().parseFromString(html, 'text/html').body;
        const blockSelector = 'p,div,h1,h2,h3,h4,h5,h6,blockquote,pre';
        const blocks = Array.from(body.querySelectorAll(blockSelector));
        const textBlocks = blocks.filter((block) => block.textContent?.trim() && !block.querySelector(blockSelector));
        const hasStructuredContent = Boolean(body.querySelector('table,ul,ol,img,svg,math,hr,canvas'));
        const hasLineBreaks = textBlocks.some((block) => block.querySelector('br'));
        if (!hasStructuredContent && !hasLineBreaks && textBlocks.length <= 1) {
          let removedEmptyBlocks = false;
          for (const block of blocks.reverse()) {
            if (!block.textContent?.trim()) { block.remove(); removedEmptyBlocks = true; }
          }
          for (const fragment of body.querySelectorAll('[data-docx-fragment-end]')) {
            // A Single Copied Text Paragraph Must Not Insert Its Paragraph Ending.
            fragment.setAttribute('data-docx-fragment-end', 'open');
            if (removedEmptyBlocks) fragment.removeAttribute('data-docx-fragment');
          }
          pastedText = singleLineText;
          html = body.innerHTML;
        }
      }
      else if (!html && /^[^\r\n]+\r?\n$/.test(text) && selection.anchor.paragraphId === selection.head.paragraphId && Math.max(selection.anchor.offset, selection.head.offset) > 0) {
        pastedText = singleLineText;
      }
      // Complete Text Paste And Selection Placement In One Event, Before Further Typing.
      const result = editor.exec({ type: 'paste', text: pastedText, ...(html ? { html } : {}) });
      if (!result.ok) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (!part || anchorIndex < 0 || headIndex < 0 || surface.session.packageRevision() === revision) return;
      const updated = surface.session.paragraphIdsIn(scope);
      const positions = new Map(updated.map((id, index) => [id, index]));
      const following = followingParagraphs.find((id) => positions.has(id));
      const lastIndex = following ? positions.get(following)! - 1 : updated.length - 1;
      const paragraphId = updated[lastIndex];
      if (!paragraphId) return;
      const updatedPart = surface.session.partFor(scope);
      if (!updatedPart) return;
      const length = paragraphTextOf(updatedPart, paragraphId)?.length;
      if (length === undefined || length < tailLength) return;
      const position = { paragraphId, offset: length - tailLength };
      surface.setSelection({ anchor: position, head: position });
      surface.revealPosition(position, { block: 'centerIfNeeded' });
    };
    window.addEventListener('paste', pasteInline, true);
    return () => window.removeEventListener('paste', pasteInline, true);
  }, [editor]);

  useEffect(() => {
    const searchMenus = (event: KeyboardEvent) => {
      if (!event.altKey || event.ctrlKey || event.metaKey || event.code !== 'Slash' || document.querySelector('dialog[open]')) return;
      event.preventDefault();
      openToolFinder();
    };
    window.addEventListener('keydown', searchMenus);
    return () => window.removeEventListener('keydown', searchMenus);
  }, [editor]);

  async function exportText(): Promise<void> {
    if (!editor || exporting) return;
    setExporting(true);
    try {
      const text = editor.query({ type: 'paragraphs' }).map((paragraph) => paragraph.text).join('\n');
      const filename = await exportPlainText(text, state.token);
      if (filename) setNotice(`Exported ${filename}`);
    } catch (error: unknown) { setNotice(error instanceof Error ? error.message : String(error)); }
    finally { setExporting(false); }
  }

  useEffect(() => {
    let disposed = false;
    const refresh = () => { void recentDocuments().then((files) => { if (!disposed) { setRecent(files); setRecentError(null); } }).catch(() => { if (!disposed) setRecentError('Recent Files Could Not Be Loaded.'); }); };
    refresh();
    window.addEventListener('insyndoc:recent-changed', refresh);
    return () => { disposed = true; window.removeEventListener('insyndoc:recent-changed', refresh); };
  }, []);

  useEffect(() => { if (props.focusMode) editor?.focus(); }, [props.focusMode, editor]);

  useEffect(() => {
    if (!editor) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let version = 0;
    let disposed = false;
    const updatePage = () => props.onPageCount(editor.snapshot().page.total);
    const scheduleStatistics = () => {
      const request = ++version;
      clearTimeout(timer);
      timer = setTimeout(() => {
        const text = editor.query({ type: 'paragraphs' }).map((paragraph) => paragraph.text).join('\n');
        void countDocumentText(text).then((statistics) => {
          if (!disposed && request === version) props.onStatistics(statistics);
        }).catch(() => undefined);
      }, 300);
    };
    updatePage(); scheduleStatistics();
    const stopSelection = editor.on('selectionChange', updatePage);
    const stopChange = editor.on('change', () => { updatePage(); scheduleStatistics(); });
    return () => { disposed = true; clearTimeout(timer); stopSelection(); stopChange(); };
  }, [editor, props.onPageCount, props.onStatistics]);

  const menus = <div className={`docx-editor docs-menu ${props.theme === 'dark' ? 'dark' : ''}`}>
    <DocxEditor.Menu preset={false} t={translate} reportIssue={false}>
      <DocxEditor.Menu.File preset={false}>
        <MenuAction icon={<Icon name="newDocument" size={18}/>} onSelect={() => void session.newDocument()} shortcut="Ctrl+N">New</MenuAction>
        <MenuAction icon={<Icon name="folder" size={18}/>} onSelect={() => void session.open()} shortcut="Ctrl+O">Open</MenuAction>
        <DocxEditor.Menu.Submenu label="Open Recent" paths={tableChromeIconPaths('file_upload')}>
          {recentError && <DocxEditor.Menu.Row disabled>{recentError}</DocxEditor.Menu.Row>}
          {recent.length === 0 && !recentError && <DocxEditor.Menu.Row disabled>No Recent Documents</DocxEditor.Menu.Row>}
          {recent.map((file) => <MenuAction icon={<Icon name="document" size={18}/>} key={file.token} disabled={!file.available} title={!file.available ? 'This File Was Moved Or Is Unavailable.' : undefined}
            onSelect={() => void session.openRecent(file.token)}><span className="recent-document"><strong>{file.name}</strong><small>{file.path}</small></span></MenuAction>)}
          {recent.length > 0 && <><DocxEditor.Menu.Separator/><MenuAction icon={<Icon name="trash" size={18}/>} onSelect={() => {
            void clearRecentDocuments().then(() => { setRecent([]); setRecentError(null); }).catch(() => setRecentError('Recent Files Could Not Be Cleared.'));
          }}>Clear Recent Files</MenuAction></>}
        </DocxEditor.Menu.Submenu>
        <DocxEditor.Menu.Separator/>
        <MenuAction icon={<Icon name="save" size={18}/>} onSelect={() => void session.save()} shortcut="Ctrl+S">Save</MenuAction>
        <MenuAction icon={<Icon name="download" size={18}/>} onSelect={() => void session.exportCopy()} shortcut="Ctrl+Shift+S">Export DOCX</MenuAction>
        <MenuAction icon={<Icon name="document" size={18}/>} disabled={exporting} onSelect={() => void exportText()}>Export Plain Text</MenuAction>
        <DocxEditor.Menu.Separator/>
        <MenuAction icon={<Icon name="document" size={18}/>} onSelect={() => setTemplatesOpen(true)}>Templates</MenuAction>
        <MenuAction icon={<Icon name="pages" size={18}/>} onSelect={() => setHistoryOpen(true)}>Version History</MenuAction>
        <DocxEditor.Menu.Separator/><DocxEditor.Menu.PageSetup/>
      </DocxEditor.Menu.File>
      <DocxEditor.Menu.Menu id="edit" label="Edit" preset={false}>
        <DocxEditor.Menu.Item slotId="history.undo" labelKey="formattingBar.undo" shortcutKey="shortcut.history.undo"/><DocxEditor.Menu.Item slotId="history.redo" labelKey="formattingBar.redo" shortcutKey="shortcut.history.redo"/>
        <DocxEditor.Menu.Separator/><MenuAction icon={<Icon name="selectAll" size={18}/>} onSelect={() => editor?.exec({ type: 'selectAll' })} shortcut="Ctrl+A">Select All</MenuAction>
        <MenuAction icon={<Icon name="search" size={18}/>} onSelect={() => props.onFind()} shortcut="Ctrl+F">Find</MenuAction>
        <MenuAction icon={<Icon name="replace" size={18}/>} onSelect={() => props.onFind(true)} shortcut="Ctrl+Shift+I">Find And Replace</MenuAction>
      </DocxEditor.Menu.Menu>
      <DocxEditor.Menu.Menu id="view" label="View" preset={false}>
        <MenuAction icon={<Icon name="pages" size={18}/>} active={state.view === 'pages'} onSelect={() => session.setView('pages')}>Separate Pages</MenuAction>
        <MenuAction icon={<Icon name="page" size={18}/>} active={state.view === 'continuous'} onSelect={() => session.setView('continuous')}>Continuous</MenuAction>
        <DocxEditor.Menu.Separator/>
        <MenuAction icon={<Icon name="outline" size={18}/>} active={outlineVisible} onSelect={toggleOutline}>Document Outline</MenuAction>
        <MenuAction icon={<Icon name="ruler" size={18}/>} active={preferences.showRuler} onSelect={() => onPreferences({ ...preferences, showRuler: !preferences.showRuler })}>Show Ruler</MenuAction>
        <DocxEditor.Menu.Item slotId="review.paragraphMarks" shortcutKey="shortcut.review.paragraphMarks"/>
        <DocxEditor.Menu.Item slotId="review.bookmarks"/>
        <MenuAction icon={<Icon name="focus" size={18}/>} active={props.focusMode} onSelect={props.onFocusMode} shortcut="Ctrl+Shift+F">Focus Mode</MenuAction>
        <DocxEditor.Menu.Separator/><MenuAction icon={<Icon name="goToPage" size={18}/>} onSelect={props.onGoToPage} shortcut="Ctrl+G">Go To Page</MenuAction>
      </DocxEditor.Menu.Menu>
      <DocxEditor.Menu.Insert>
        <DocxEditor.Menu.Item slotId="insert.footnote" shortcutKey="shortcut.insert.footnote"/>
        <DocxEditor.Menu.Item slotId="insert.endnote" shortcutKey="shortcut.insert.endnote"/>
        <DocxEditor.Menu.Item slotId="insert.pageBreak" shortcutKey="shortcut.insert.pageBreak"/>
        <DocxEditor.Menu.Separator/>
        <MenuAction icon={<Icon name="calendar" size={18}/>} onSelect={() => setWritingTool('date')}>Date And Time</MenuAction>
        <MenuAction icon={<Icon name="symbols" size={18}/>} onSelect={() => setWritingTool('symbols')}>Special Characters</MenuAction>
        <MenuAction icon={<Icon name="math" size={18}/>} onSelect={() => setWritingTool('math')}>Math Equation</MenuAction>
        <MenuAction icon={<Icon name="table" size={18}/>} onSelect={() => setWritingTool('table')}>Custom Table</MenuAction>
        <DocxEditor.Menu.Submenu label="Headers And Footers" paths={tableChromeIconPaths('vertical_align_top')}>
          <MenuAction icon={<Icon name="header" size={18}/>} onSelect={() => editFurniture('header')}>Edit Header</MenuAction>
          <MenuAction icon={<Icon name="footer" size={18}/>} onSelect={() => editFurniture('footer')}>Edit Footer</MenuAction>
          <MenuAction icon={<Icon name="document" size={18}/>} disabled={!furniture?.editing} onSelect={() => editor?.exec({ type: 'exitHeaderFooter' })}>Return To Document</MenuAction>
        </DocxEditor.Menu.Submenu>
        <DocxEditor.Menu.Submenu label="Page Number Fields" paths={tableChromeIconPaths('format_list_numbered')}>
          <DocxEditor.Menu.Item slotId="insert.pageNumber"/>
          <DocxEditor.Menu.Item slotId="insert.totalPages"/>
          <DocxEditor.Menu.Item slotId="insert.pageXofY"/>
        </DocxEditor.Menu.Submenu>
      </DocxEditor.Menu.Insert>
      <DocxEditor.Menu.Format>
        <DocxEditor.Menu.Item slotId="text.bold" labelKey="formattingBar.bold" shortcutKey="shortcut.text.bold"/>
        <DocxEditor.Menu.Item slotId="text.italic" labelKey="formattingBar.italic" shortcutKey="shortcut.text.italic"/>
        <DocxEditor.Menu.Item slotId="text.underline" labelKey="formattingBar.underline" shortcutKey="shortcut.text.underline"/>
        <DocxEditor.Menu.Item slotId="alignment.left" shortcutKey="shortcut.alignment.left"/>
        <DocxEditor.Menu.Item slotId="alignment.center" shortcutKey="shortcut.alignment.center"/>
        <DocxEditor.Menu.Item slotId="alignment.right" shortcutKey="shortcut.alignment.right"/>
        <DocxEditor.Menu.Item slotId="alignment.justify" shortcutKey="shortcut.alignment.justify"/>
        <DocxEditor.Menu.Item slotId="direction.ltr" shortcutKey="shortcut.direction.ltr"/>
        <DocxEditor.Menu.Item slotId="direction.rtl" shortcutKey="shortcut.direction.rtl"/>
        <DocxEditor.Menu.Item slotId="format.painter" labelKey="formattingBar.formatPainter" shortcutKey="shortcut.format.painter"/>
        <DocxEditor.Menu.Separator/>
        <DocxEditor.Menu.Submenu label="Text Position" paths={tableChromeIconPaths('superscript')}>
          <DocxEditor.Menu.Item slotId="script.super" labelKey="formattingBar.superscript" shortcutKey="shortcut.script.super"/>
          <DocxEditor.Menu.Item slotId="script.sub" labelKey="formattingBar.subscript" shortcutKey="shortcut.script.sub"/>
        </DocxEditor.Menu.Submenu>
      </DocxEditor.Menu.Format>
      <DocxEditor.Menu.Menu id="tools" label="Tools" preset={false}>
        <MenuAction icon={<Icon name="spelling" size={18}/>} onSelect={() => setSpellingRequest((value) => value + 1)} shortcut="Ctrl+Alt+X" disabled={!preferences.checkSpelling}>Check Spelling</MenuAction>
        <MenuAction icon={<Icon name="wordCount" size={18}/>} onSelect={props.onWordCount}>Word Count</MenuAction>
        <MenuAction icon={<Icon name="keyboard" size={18}/>} onSelect={props.onShortcuts} shortcut="Ctrl+/">Keyboard Shortcuts</MenuAction>
        <MenuAction icon={<Icon name="replace" size={18}/>} onSelect={() => props.onFind(true)} shortcut="Ctrl+Shift+I">Find And Replace</MenuAction>
        <DocxEditor.Menu.Separator/><MenuAction icon={<Icon name="settings" size={18}/>} onSelect={props.onSettings} shortcut="Ctrl+,">Settings</MenuAction>
      </DocxEditor.Menu.Menu>
      <DocxEditor.Menu.Menu id="help" label="Help" preset={false}>
        <MenuAction icon={<Icon name="search" size={18}/>} onSelect={() => { openToolFinder(); }} shortcut="Alt+/">Search The Menus</MenuAction>
        <MenuAction icon={<Icon name="keyboard" size={18}/>} onSelect={props.onShortcuts} shortcut="Ctrl+/">Keyboard Shortcuts</MenuAction>
      </DocxEditor.Menu.Menu>
    </DocxEditor.Menu>
  </div>;

  return <>
    {props.menuHost && createPortal(menus, props.menuHost)}
    <div className="toolbar-shell"><DocxEditor.Toolbar t={translate} className="docs-toolbar" onSave={() => void session.save()}>
      {['history', 'zoom', 'styles', 'font', 'text', 'script', 'alignment', 'list', 'format', 'review'].map((id) => <DocxEditor.Toolbar.Group key={id} id={id} hidden/>)}
      <DocxEditor.Toolbar.Group id="docs-search" label="Search" pinned>
        <button className="toolbar-menu-search" aria-label="Menus" title="Search The Menus (Alt+/)" onClick={() => { openToolFinder(); }}><Icon name="search" size={17}/></button>
      </DocxEditor.Toolbar.Group>
      <DocxEditor.Toolbar.Group id="docs-history" label="History" after="docs-search"><DocxEditor.Toolbar.Undo/><DocxEditor.Toolbar.Redo/><DocxEditor.Toolbar.Button slotId="format.painter"/></DocxEditor.Toolbar.Group>
      {props.zoomHost && createPortal(<DocxEditor.Toolbar.Zoom/>, props.zoomHost)}
      <DocxEditor.Toolbar.Group id="docs-style" label="Paragraph Style" after="docs-history"><ParagraphStylePicker/></DocxEditor.Toolbar.Group>
      <DocxEditor.Toolbar.Group id="docs-font" label="Font" after="docs-style"><DocxEditor.Toolbar.FontFamily/><DocxEditor.Toolbar.FontSize/></DocxEditor.Toolbar.Group>
      <DocxEditor.Toolbar.Group id="docs-text" label="Text Formatting" after="docs-font"><DocxEditor.Toolbar.Bold/><DocxEditor.Toolbar.Italic/><DocxEditor.Toolbar.Underline/><DocxEditor.Toolbar.FontColor/><DocxEditor.Toolbar.Highlight/></DocxEditor.Toolbar.Group>
      <DocxEditor.Toolbar.Group id="docs-insert" label="Insert" after="docs-paragraph"><DocxEditor.Toolbar.Link/><DocxEditor.Toolbar.ImageInsert/><DocxEditor.Toolbar.TableInsert/></DocxEditor.Toolbar.Group>
      <DocxEditor.Toolbar.Group id="docs-paragraph" label="Paragraph Formatting" after="docs-text"><DocxEditor.Toolbar.Alignment/><DocxEditor.Toolbar.LineSpacing/><DocxEditor.Toolbar.BulletList/><DocxEditor.Toolbar.NumberedList/><DocxEditor.Toolbar.Outdent/><DocxEditor.Toolbar.Indent/><DocxEditor.Toolbar.ClearFormatting/></DocxEditor.Toolbar.Group>
      <DocxEditor.Toolbar.Group id="docs-script" label="More Text Formatting" after="docs-insert"><DocxEditor.Toolbar.Strike/><DocxEditor.Toolbar.Superscript/><DocxEditor.Toolbar.Subscript/></DocxEditor.Toolbar.Group>
    </DocxEditor.Toolbar><div className="toolbar-end">
      {!props.focusMode && <button className={`icon-button ${outlineVisible ? 'active' : ''}`} aria-label="Toggle Document Outline" aria-pressed={outlineVisible} title="Document Outline" onClick={toggleOutline}><Icon name="outline" size={18}/></button>}
      <button className="icon-button" aria-label="Hide Menus" title="Hide Menus (Ctrl+Shift+F)" onClick={props.onFocusMode}><Icon name="arrowUp" size={18}/></button>
    </div></div>
    <Modal open={menuSearchOpen} title="Search The Menus" onClose={() => setMenuSearchOpen(false)} className="menu-search-modal">
      <div className="menu-search-input"><Icon name="search" size={19}/><input autoFocus aria-label="Search The Menus" placeholder="Search For A Tool Or Action" value={menuQuery} onChange={(event) => setMenuQuery(event.target.value)} onKeyDown={(event) => {
        if (event.key === 'ArrowDown') { event.preventDefault(); document.querySelector<HTMLButtonElement>('.menu-search-results button')?.focus(); return; }
        if (event.key !== 'Enter' || !menuCommands.length) return;
        event.preventDefault(); runMenuCommand(menuCommands[0].run);
      }}/><kbd>Alt+/</kbd></div>
      <div className="menu-search-results" onKeyDown={(event) => {
        const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('button'));
        const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
        let next: number;
        if (event.key === 'ArrowDown') next = (current + 1) % buttons.length;
        else if (event.key === 'ArrowUp') next = (current - 1 + buttons.length) % buttons.length;
        else if (event.key === 'Home') next = 0;
        else if (event.key === 'End') next = buttons.length - 1;
        else return;
        event.preventDefault();
        buttons[next]?.focus();
      }}>{menuCommands.map((command) => <button key={command.label} onClick={() => { runMenuCommand(command.run); }}><span>{command.label}</span><kbd>{command.shortcut}</kbd></button>)}{menuCommands.length === 0 && <p>No Matching Tools. Try Another Search.</p>}</div>
    </Modal>
    <Spelling preferences={preferences} onPreferences={onPreferences} request={spellingRequest} documentKey={`${state.token}:${state.busy}`}/>
    <WritingTools tool={writingTool} onClose={() => setWritingTool(null)}/>
    {templatesOpen && <TemplateGallery session={session} onClose={() => setTemplatesOpen(false)}/>}
    {historyOpen && <VersionHistory session={session} onClose={() => setHistoryOpen(false)}/>}
    {furniture?.editing && <div className="header-footer-strip"><span>Editing {furniture.editing === 'header' ? 'Header' : 'Footer'}</span><span>Insert Page Numbers From The Insert Menu.</span><button onClick={() => editor?.exec({ type: 'exitHeaderFooter' })}>Return To Document</button></div>}
    {props.findOpen && <FindPanel replaceMode={props.replaceMode} request={props.findRequest} onClose={props.onCloseFind}/>}
    <div className="workspace-body">
      {outlineVisible && !props.focusMode && <><button className="outline-backdrop" aria-label="Close Document Outline" onClick={closeOutline}/><Outline outline={outline} onClose={closeOutline}/></>}
      <div className="document-stage">
        {preferences.showRuler && <div className="ruler-wrap"><DocxEditor.HorizontalRuler unit={preferences.rulerUnit}/></div>}
        <DocxEditor.Viewport className="document-viewport">
          <DocxEditor.Content/>
          <DocxEditor.HyperLink/><DocxEditor.ContentControl/><DocxEditor.Equation/><DocxEditor.ContextMenu/>
          <DocxEditor.OpenFailure/>
        </DocxEditor.Viewport>
      </div>
    </div>
    {notice && <div className="workspace-notice" role="status"><span>{notice}</span><button className="icon-button" aria-label="Dismiss Notification" onClick={() => setNotice(null)}><Icon name="close" size={16}/></button></div>}
  </>;
}

function EditorWorkspace(props: WorkspaceProps) {
  const blankDocument = useMemo(createBlankDocument, []);
  return <div className={`docx-editor docs-editor ${props.theme === 'dark' ? 'dark' : ''}`}>
    <DocxEditor.Root document={blankDocument} fonts={localFonts} mode="edit" colorMode={props.theme} translate={translate}
      onReady={(editor) => props.onReady(editor, createEditorAdapter(editor))}
      onChange={(change) => { if (!change.source) props.session.changed(); }}>
      <WorkspaceContent {...props}/>
    </DocxEditor.Root>
  </div>;
}

export default memo(EditorWorkspace);
