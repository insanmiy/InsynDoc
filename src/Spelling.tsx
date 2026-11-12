import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useDocxEditor } from '@docx-editor.dev/react';
import { paragraphOrderOfPart, paragraphTextOf } from '@docx-editor.dev/core/store';
import { replaceDocumentMatches } from './document-replacement';
import { isSpellingWord, spellingKey, type SpellingIssue, type SpellingParagraph, type SpellingRequest, type SpellingResponse } from './spelling-engine';
import { learnSpellingCorrection } from './spelling-learning';
import type { Preferences } from './preferences';
import { Icon } from './Icon';

interface SuggestionPopup {
  issue: SpellingIssue | null;
  left: number;
  top: number;
  suggestions: string[] | null;
  message?: string;
  focus?: boolean;
}

export function Spelling({ preferences, onPreferences, request, documentKey }: {
  preferences: Preferences;
  onPreferences: (preferences: Preferences) => void;
  request: number;
  documentKey: string;
}) {
  const editor = useDocxEditor();
  const [ignored, setIgnored] = useState<string[]>([]);
  const [ignoredGrammar, setIgnoredGrammar] = useState<string[]>([]);
  const [retry, setRetry] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [truncated, setTruncated] = useState(false);
  const [popup, setPopup] = useState<SuggestionPopup | null>(null);
  const [customWord, setCustomWord] = useState<string | null>(null);
  const learnedCorrections = useRef(preferences.spellingCorrections);
  const scheduleCheck = useRef<() => void>(() => undefined);
  const popupElement = useRef<HTMLDivElement>(null);
  const openFirst = useRef<() => void>(() => undefined);
  const lastRequest = useRef(0);
  const reviewPending = useRef(false);
  const focusedButton = useRef<HTMLButtonElement | null>(null);
  const dismiss = useCallback(() => {
    reviewPending.current = false;
    setPopup(null);
    setCustomWord(null);
  }, []);

  useEffect(() => {
    learnedCorrections.current = preferences.spellingCorrections;
    scheduleCheck.current();
  }, [preferences.spellingCorrections]);

  useEffect(() => {
    const escape = (event: KeyboardEvent): void => {
      if (event.key !== 'Escape' || !popupElement.current || document.querySelector('dialog[open]')) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      dismiss();
      editor?.focus();
    };
    window.addEventListener('keydown', escape, true);
    return () => window.removeEventListener('keydown', escape, true);
  }, [editor, dismiss]);

  useEffect(() => {
    dismiss();
    setError(null);
    setTruncated(false);
    if (!editor || !preferences.checkSpelling) {
      editor?.clearHighlights('spelling');
      openFirst.current = () => setPopup({ issue: null, left: 24, top: 140, suggestions: [], message: 'Turn On Check Spelling In Settings To Check This Document.' });
      return;
    }
    let worker: Worker | null = null;
    let disposed = false;
    let version = 0;
    let suggestionVersion = 0;
    let inFlight = false;
    let queued = false;
    let checked = false;
    let timer: ReturnType<typeof setTimeout>;
    let issues: SpellingIssue[] = [];
    const excluded = [...preferences.spellingWords, ...ignored];
    const post = (message: SpellingRequest): void => {
      if (!worker) {
        worker = new Worker(new URL('./spelling.worker.ts', import.meta.url), { type: 'module' });
        worker.onerror = unavailable;
        worker.onmessage = onWorkerMessage;
      }
      worker.postMessage(message);
    };
    const position = (rect?: { left: number; bottom: number }) => ({
      left: Math.max(8, Math.min(rect?.left ?? window.innerWidth - 320, window.innerWidth - 304)),
      top: Math.max(8, Math.min(rect?.bottom ? rect.bottom + 8 : 150, window.innerHeight - 320))
    });
    const openIssue = (issue: SpellingIssue, rect?: { left: number; bottom: number }, focus = false): void => {
      reviewPending.current = false;
      setCustomWord(null);
      setPopup({ issue, suggestions: issue.suggestions ?? null, focus, ...position(rect) });
      suggestionVersion++;
      if (!issue.suggestions) post({ type: 'suggest', id: suggestionVersion, word: issue.word, personalWords: preferences.spellingWords, corrections: learnedCorrections.current });
    };
    openFirst.current = () => {
      if (!checked) {
        reviewPending.current = true;
        setPopup({ issue: null, suggestions: [], message: 'Checking Spelling...', focus: true, ...position() });
        return;
      }
      if (issues[0]) {
        const issue = issues[0];
        const issueVersion = version;
        editor.scrollToBlock(issue.blockId);
        requestAnimationFrame(() => { if (!disposed && issueVersion === version) openIssue(issue, undefined, true); });
        return;
      }
      setPopup({ issue: null, suggestions: [], message: 'No Spelling Issues Found In English (US). Sentence Context Checked Too.', focus: true, ...position() });
    };
    const check = (): void => {
      if (disposed) return;
      if (inFlight) { queued = true; return; }
      const surface = editor.surface;
      if (!surface) return;
      const paragraphs: SpellingParagraph[] = [];
      for (const part of surface.session.storyParts()) {
        for (const blockId of paragraphOrderOfPart(part).keys()) {
          const text = paragraphTextOf(part, blockId);
          if (text) paragraphs.push({ blockId, text });
        }
      }
      if (paragraphs.length === 0 && !worker) {
        checked = true;
        issues = [];
        setTruncated(false);
        editor.clearHighlights('spelling');
        if (reviewPending.current) { reviewPending.current = false; openFirst.current(); }
        return;
      }
      inFlight = true;
      queued = false;
      post({ type: 'check', id: version, paragraphs, excluded: [...excluded, ...learnedCorrections.current.map((entry) => entry.replacement)], ignoredGrammar });
    };
    const schedule = (): void => {
      checked = false;
      queued = false;
      version++;
      dismiss();
      clearTimeout(timer);
      timer = setTimeout(check, 450);
    };
    scheduleCheck.current = schedule;
    const unavailable = (): void => {
      if (disposed) return;
      editor.clearHighlights('spelling');
      dismiss();
      setError('Spelling Is Unavailable. Your Document Can Still Be Edited.');
      openFirst.current = () => setPopup({ issue: null, suggestions: [], message: 'Spelling Is Unavailable. Use Retry Spelling To Try Again.', focus: true, ...position() });
    };
    const onWorkerMessage = (event: MessageEvent<SpellingResponse>): void => {
      if (disposed) return;
      const result = event.data;
      if (result.type === 'error') { unavailable(); return; }
      if (result.type === 'suggested') {
        if (result.id === suggestionVersion) setPopup((current) => current?.issue ? { ...current, suggestions: result.suggestions } : current);
        return;
      }
      inFlight = false;
      if (result.id === version) {
        checked = true;
        issues = result.issues;
        setTruncated(result.truncated);
        editor.setHighlights('spelling', issues, { className: 'spelling-mark', color: 'transparent', priority: 1 });
        if (reviewPending.current) { reviewPending.current = false; openFirst.current(); }
      }
      if (queued) check();
    };
    const pick = (event: MouseEvent): void => {
      if (event.target instanceof Node && popupElement.current?.contains(event.target)) return;
      if (event.ctrlKey || event.metaKey || event.altKey || document.querySelector('dialog[open]')) { dismiss(); return; }
      const hit = editor.getHighlightsAt<SpellingIssue>(event.clientX, event.clientY).find((candidate) => candidate.name === 'spelling');
      if (!hit) { dismiss(); return; }
      event.preventDefault();
      event.stopImmediatePropagation();
      openIssue({ ...hit.range, start: hit.start, length: hit.length }, hit.rect);
    };
    const keydown = (event: KeyboardEvent): void => {
      if (document.querySelector('dialog[open]')) return;
      if ((event.ctrlKey || event.metaKey) && event.altKey && event.key.toLowerCase() === 'x') {
        event.preventDefault(); openFirst.current();
      }
    };
    const scrolled = (event: Event): void => {
      if (!(event.target instanceof Node) || !popupElement.current?.contains(event.target)) dismiss();
    };
    const resized = (): void => dismiss();
    const stopChange = editor.on('change', schedule);
    document.addEventListener('click', pick, true);
    document.addEventListener('contextmenu', pick, true);
    document.addEventListener('keydown', keydown);
    document.addEventListener('scroll', scrolled, true);
    window.addEventListener('resize', resized);
    const runWhenIdle = window.requestIdleCallback ?? ((callback: () => void): number => window.setTimeout(callback, 2000));
    const cancelWhenIdle = window.cancelIdleCallback ?? ((handle: number): void => window.clearTimeout(handle));
    const idleHandle = runWhenIdle(() => schedule());
    return () => {
      disposed = true;
      scheduleCheck.current = () => undefined;
      cancelWhenIdle(idleHandle);
      clearTimeout(timer);
      worker?.terminate();
      stopChange();
      editor.clearHighlights('spelling');
      document.removeEventListener('click', pick, true);
      document.removeEventListener('contextmenu', pick, true);
      document.removeEventListener('keydown', keydown);
      document.removeEventListener('scroll', scrolled, true);
      window.removeEventListener('resize', resized);
    };
  }, [editor, preferences.checkSpelling, preferences.spellingWords, ignored, ignoredGrammar, documentKey, retry, dismiss]);

  useEffect(() => {
    if (request === lastRequest.current) return;
    lastRequest.current = request;
    openFirst.current();
  }, [request]);

  useLayoutEffect(() => {
    if (!popup || !popupElement.current) return;
    const bounds = popupElement.current.getBoundingClientRect();
    const left = Math.max(8, Math.min(popup.left, window.innerWidth - bounds.width - 8));
    const top = Math.max(8, Math.min(popup.top, window.innerHeight - bounds.height - 8));
    if (left !== popup.left || top !== popup.top) setPopup((current) => current ? { ...current, left, top } : null);
  }, [popup?.left, popup?.top, popup?.suggestions, customWord !== null]);

  useEffect(() => {
    if (!popup?.focus) { focusedButton.current = null; return; }
    if (focusedButton.current && document.activeElement !== focusedButton.current) return;
    const button = popupElement.current?.querySelector<HTMLButtonElement>('.spelling-suggestions button')
      ?? popupElement.current?.querySelector<HTMLButtonElement>('.spelling-heading button');
    if (button) { button.focus(); focusedButton.current = button; }
  }, [popup?.issue?.word, Boolean(popup), popup?.focus, popup?.suggestions]);

  function correctWord(text: string): void {
    if (!editor || !popup?.issue) return;
    const issue = popup.issue;
    const match = editor.findMatches(issue.word, { matchCase: true, wholeWord: issue.kind !== 'grammar' })
      .find((candidate) => candidate.blockId === issue.blockId && candidate.start === issue.start && candidate.length === issue.length);
    dismiss();
    if (!match) return;
    const replacementError = replaceDocumentMatches(editor, [match], text);
    setError(replacementError);
    if (!replacementError && issue.kind !== 'grammar') {
      onPreferences({ ...preferences, spellingCorrections: learnSpellingCorrection(preferences.spellingCorrections, issue.word, text) });
    }
    editor.focus();
  }

  function close(): void {
    dismiss();
    editor?.focus();
  }

  return <>
    {error && <div className="spelling-error" role="status"><span>{error}</span><button className="text-button" onClick={() => setRetry((value) => value + 1)}>Retry Spelling</button></div>}
    {popup && createPortal(<div ref={popupElement} className="spelling-popover" role="dialog" aria-label="Spelling Suggestions" style={{ left: popup.left, top: popup.top }} onKeyDown={(event) => {
      if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(); }
    }}>
      <div className="spelling-heading"><strong>{popup.issue?.word ?? 'Spelling'}</strong><button className="icon-button" aria-label="Close Spelling Suggestions" onClick={close}><Icon name="close" size={16}/></button></div>
      {truncated && <p role="status" className="spelling-caption">Showing The First 1,000 Spelling Issues. Fix Some To See More.</p>}
      {popup.issue ? <>
        <p className="spelling-caption">{popup.issue.kind === 'grammar' ? 'Grammar Suggestion' : 'Spelling Suggestion'}</p>
        {popup.issue.message && <p className="spelling-caption">{popup.issue.message}</p>}
        <div className="spelling-suggestions">{popup.suggestions === null ? <p role="status">Finding Suggestions...</p> : popup.suggestions.length ? popup.suggestions.map((suggestion) => <button key={suggestion} onClick={() => correctWord(suggestion)}>{suggestion || 'Remove Text'}</button>) : <p>No Suggestions Found.</p>}</div>
        {popup.issue.kind !== 'grammar' && <>
          <p className="spelling-caption">Suggestions Learn From Corrections You Accept On This Device.</p>
          {customWord !== null && <form className="spelling-custom" onSubmit={(event) => {
            event.preventDefault();
            const replacement = customWord.trim();
            if (isSpellingWord(replacement) && spellingKey(replacement) !== spellingKey(popup.issue!.word)) correctWord(replacement);
          }}>
            <label htmlFor="spelling-custom-word">Correct Word</label>
            <input id="spelling-custom-word" autoFocus autoComplete="off" maxLength={64} value={customWord} onChange={(event) => setCustomWord(event.target.value)}/>
            <button type="submit" className="secondary-button" disabled={!isSpellingWord(customWord.trim()) || spellingKey(customWord.trim()) === spellingKey(popup.issue.word)}>Apply Correction</button>
          </form>}
        </>}
        {popup.issue.kind !== 'grammar' && <div className="spelling-actions"><button onClick={() => setCustomWord('')}>No Words Match</button></div>}
        <div className="spelling-actions"><button onClick={() => { if (popup.issue!.ignoreKey) setIgnoredGrammar((keys) => [...keys, popup.issue!.ignoreKey!]);
          else setIgnored((words) => [...words, popup.issue!.word]);
          close(); }}>Ignore For This Session</button>{popup.issue.kind !== 'grammar' && <button disabled={preferences.spellingWords.length >= 5000} onClick={() => {
          const word = popup.issue!.word;
          if (!preferences.spellingWords.some((saved) => spellingKey(saved) === spellingKey(word))) onPreferences({ ...preferences, spellingWords: [...preferences.spellingWords, word] });
          close();
        }}>Add To Dictionary</button>}</div>
      </> : <p role="status" className="spelling-caption">{popup.message}</p>}
    </div>, document.body)}
  </>;
}
