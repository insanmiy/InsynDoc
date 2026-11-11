import type { DocxEditorInstance } from '@docx-editor.dev/core/editor';
import { directParagraphProperties, ensureListDefinition, findNode, indexStyles, mergedParagraphMarkProperties, mergedProperties, paragraphTextOf, runPropertyEdits, stylesPartOf, type TreeDocOp } from '@docx-editor.dev/core/store';
import { headingFontSize } from './paragraph-styles';

type AutoformatRule = { kind: 'bullet' | 'ordered'; prefix: string } | { kind: 'heading'; prefix: string; level: number };

export function matchAutoformat(prefix: string): AutoformatRule | null {
  if (/^[*+-]$/.test(prefix)) return { kind: 'bullet', prefix };
  if (/^1[.)]$/.test(prefix)) return { kind: 'ordered', prefix };
  if (/^#{1,6}$/.test(prefix)) return { kind: 'heading', prefix, level: prefix.length };
  return null;
}

function installHeadingContinuation(editor: DocxEditorInstance): () => void {
  const surface = editor.surface;
  if (!surface) return () => undefined;
  const host = surface.hostElement();
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== 'Enter' || event.shiftKey || event.ctrlKey || event.altKey || event.metaKey || event.isComposing || event.defaultPrevented) return;
    if (!(event.target instanceof HTMLElement) || !event.target.closest('.docx-pages')) return;
    if (surface.editingMode() !== 'edit' || surface.collaborationSession() || !editor.snapshot().editable) return;
    surface.flushPendingInput();
    const selection = surface.state().selection;
    const { head, anchor } = selection;
    if (head.paragraphId !== anchor.paragraphId || head.offset !== anchor.offset || surface.isListParagraph()) return;
    const scope = surface.storyScope();
    const part = surface.session.partFor(scope);
    if (!part || paragraphTextOf(part, head.paragraphId)?.length !== head.offset) return;
    const styleId = surface.formatting().styleId;
    if (!styleId || !headingFontSize(styleId)) return;
    const stylePart = stylesPartOf(surface.session.currentPackage());
    if (!stylePart) return;
    const definitions = indexStyles(stylePart);
    const definition = definitions.get(styleId);
    const node = definition ? findNode(stylePart, definition.nodeId) : null;
    const next = node && 'children' in node ? node.children.find((child) => 'localName' in child && child.localName === 'next') : undefined;
    const nextId = next && 'attributes' in next ? next.attributes.find((attribute) => attribute.localName === 'val')?.value : undefined;
    if (!nextId || nextId === styleId || headingFontSize(nextId)) return;
    const nextStyle = editor.getDocumentStyles().find((style) => style.styleId === nextId && style.type === 'paragraph');
    const size = nextStyle?.preview.fontSizePt;
    if (!size || !editor.can({ type: 'setParagraphStyle', styleId: nextId }).ok) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    // Let Enter Commit The Body Size With Its Split And Restore Both Together On Undo.
    surface.setRunProperty('sz', { val: String(Math.round(size * 2)) });
    surface.splitParagraph();
  };
  host.addEventListener('keydown', onKeyDown, true);
  return () => host.removeEventListener('keydown', onKeyDown, true);
}

function installEmptyListExit(editor: DocxEditorInstance): () => void {
  const surface = editor.surface;
  if (!surface) return () => undefined;
  const host = surface.hostElement();
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== 'Backspace' || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey || event.isComposing || event.defaultPrevented) return;
    if (!(event.target instanceof HTMLElement) || !event.target.closest('.docx-pages')) return;
    if (surface.editingMode() !== 'edit' || surface.collaborationSession() || !editor.snapshot().editable) return;
    surface.flushPendingInput();
    const { anchor, head } = surface.state().selection;
    if (anchor.paragraphId !== head.paragraphId || anchor.offset !== 0 || head.offset !== 0 || !surface.isListParagraph()) return;
    const part = surface.session.partFor(surface.storyScope());
    if (!part || paragraphTextOf(part, head.paragraphId) !== '') return;
    if (!surface.exitListOnEmptyItem()) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  };
  host.addEventListener('keydown', onKeyDown, true);
  return () => host.removeEventListener('keydown', onKeyDown, true);
}

export interface AutoformatOptions {
  smartPunctuation?: boolean;
  snippets?: readonly { trigger: string; text: string }[];
}

export interface SnippetExpansion {
  trigger: string;
  text: string;
}

type Surface = NonNullable<DocxEditorInstance['surface']>;
type CaretScope = ReturnType<Surface['storyScope']>;
type CaretPart = NonNullable<ReturnType<Surface['session']['partFor']>>;

interface CaretContext {
  scope: CaretScope;
  part: CaretPart;
  paragraphId: string;
  offset: number;
  before: string;
}

function caretContext(surface: Surface, maxOffset: number, minOffset = 1): CaretContext | null {
  const pending = surface.state().selection;
  if (pending.anchor.paragraphId !== pending.head.paragraphId || pending.anchor.offset !== pending.head.offset || pending.head.offset > maxOffset) return null;
  surface.flushPendingInput();
  const selection = surface.state().selection;
  if (selection.anchor.paragraphId !== selection.head.paragraphId || selection.anchor.offset !== selection.head.offset
    || selection.head.offset < minOffset || selection.head.offset > maxOffset) return null;
  const scope = surface.storyScope();
  const part = surface.session.partFor(scope);
  if (!part || surface.isListParagraph()) return null;
  const paragraphId = selection.head.paragraphId;
  const before = paragraphTextOf(part, paragraphId);
  if (before === null) return null;
  return { scope, part, paragraphId, offset: selection.head.offset, before };
}

function smartPunctuationReplacement(prefix: string, typed: string): string | null {
  if (typed === '-') {
    return prefix.endsWith('-') ? '—' : null;
  }
  if (typed !== '"' && typed !== "'") return null;
  const previous = prefix.charAt(prefix.length - 1);
  const opening = previous === '' || /[\s([{<"'‘“«—–-]/.test(previous);
  if (typed === '"') return opening ? '“' : '”';
  return opening ? '‘' : '’';
}

function matchSnippet(prefix: string, snippets: readonly SnippetExpansion[]): SnippetExpansion | null {
  let best: SnippetExpansion | null = null;
  for (const snippet of snippets) {
    if (!prefix.endsWith(snippet.trigger)) continue;
    const before = prefix.length - snippet.trigger.length;
    if (before > 0 && !/\s/.test(prefix.charAt(before - 1))) continue;
    if (!best || snippet.trigger.length > best.trigger.length) best = snippet;
  }
  return best;
}

export function installAutoformat(editor: DocxEditorInstance, options: AutoformatOptions = {}): () => void {
  const surface = editor.surface;
  if (!surface) return () => undefined;
  const host = surface.hostElement();
  const snippets = options.snippets ?? [];
  const uninstallHeadingContinuation = installHeadingContinuation(editor);
  let disposed = false;
  let reversibleConversion: { paragraphId: string; scope: string; revision: number } | null = null;
  const clearReversibleConversion = (): void => { reversibleConversion = null; };
  const onKeyDown = (event: KeyboardEvent): void => {
    const conversion = reversibleConversion;
    clearReversibleConversion();
    if (!conversion || event.key !== 'Backspace' || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey || event.isComposing || event.defaultPrevented) return;
    if (!(event.target instanceof HTMLElement) || !event.target.closest('.docx-pages')) return;
    if (editor.surface !== surface || surface.editingMode() !== 'edit' || !editor.snapshot().editable) return;
    surface.flushPendingInput();
    const { anchor, head } = surface.state().selection;
    if (surface.session.packageRevision() !== conversion.revision || JSON.stringify(surface.storyScope()) !== conversion.scope
      || anchor.paragraphId !== conversion.paragraphId || head.paragraphId !== conversion.paragraphId || anchor.offset !== 0 || head.offset !== 0) return;
    if (!editor.can({ type: 'undo' }).ok) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    editor.exec({ type: 'undo' });
  };
  const pendingConversions = new Set<() => void>();

  const scheduleReplacement = (event: InputEvent, target: HTMLElement, context: CaretContext, start: number, end: number, text: string, expected: string): void => {
    const { paragraphId, offset, scope } = context;
    const cleanup = (): void => {
      target.removeEventListener('beforeinput', finish);
      pendingConversions.delete(cleanup);
    };
    const finish = (received: Event): void => {
      cleanup();
      if (received !== event) return;
      if (disposed || editor.surface !== surface || JSON.stringify(surface.storyScope()) !== JSON.stringify(scope)) return;
      surface.flushPendingInput();
      const current = surface.state().selection;
      if (current.anchor.paragraphId !== paragraphId || current.head.paragraphId !== paragraphId
        || current.anchor.offset !== offset + 1 || current.head.offset !== offset + 1 || surface.isListParagraph()) return;
      const currentPart = surface.session.partFor(scope);
      if (!currentPart || paragraphTextOf(currentPart, paragraphId) !== expected) return;
      const after = { paragraphId, start: start + text.length, end: start + text.length };
      let converted = false;
      const rejection = surface.commitReviewOps((historyToken) => {
        const result = surface.session.applyTreeOps([
          { op: 'deleteText', paragraphId, start, end },
          { op: 'insertText', paragraphId, offset: start, text }
        ], { paragraphId, start: end, end }, after, scope, { historyToken });
        converted = result.committed;
        return result;
      }, 'package-scoped');
      if (!rejection && converted) {
        surface.setSelection({ anchor: { paragraphId, offset: start + text.length }, head: { paragraphId, offset: start + text.length } });
        reversibleConversion = { paragraphId, scope: JSON.stringify(scope), revision: surface.session.packageRevision() };
      }
    };
    pendingConversions.add(cleanup);
    target.addEventListener('beforeinput', finish, { once: true });
  };

  const beforeInput = (event: InputEvent): void => {
    clearReversibleConversion();
    if (event.inputType !== 'insertText' || !event.data || event.isComposing || event.defaultPrevented) return;
    const data = event.data;
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    if (event.target instanceof Element && event.target.closest('input, textarea, select')) return;
    if (surface.editingMode() !== 'edit' || surface.collaborationSession() || !editor.snapshot().editable) return;
    const isSpace = data === ' ';
    const isPunctuation = options.smartPunctuation === true && data.length === 1 && (data === '-' || data === '"' || data === "'");
    if (!isSpace && !isPunctuation) return;
    const context = caretContext(surface, 4096, isSpace ? 1 : 0);
    if (!context) return;
    const { before, offset, paragraphId, scope } = context;
    const committed = before.slice(0, offset) + data + before.slice(offset);
    const prefix = before.slice(0, offset);

    if (isSpace && snippets.length) {
      const snippet = matchSnippet(prefix, snippets);
      if (snippet) {
        const start = offset - snippet.trigger.length;
        scheduleReplacement(event, target, context, start, offset, snippet.text, committed);
        return;
      }
    }

    if (isPunctuation) {
      const replacement = smartPunctuationReplacement(prefix, data);
      if (!replacement) return;
      const start = data === '-' ? offset - 1 : offset;
      const end = offset + 1;
      if (start < 0) return;
      scheduleReplacement(event, target, context, start, end, replacement, committed);
      return;
    }

    if (offset > 6) return;
    const rule = matchAutoformat(prefix);
    if (!rule) return;
    const headingStyle = rule.kind === 'heading' ? editor.getDocumentStyles().find((style) => style.type === 'paragraph'
      && (style.styleId.toLowerCase() === `heading${rule.level}` || style.name.toLowerCase() === `heading ${rule.level}`)) : undefined;
    if (rule.kind === 'heading' && !headingStyle) return;

    // Run After The Editor's Input Handler Commits The Space, In A Separate Undo Step.
    const cleanup = (): void => {
      target.removeEventListener('beforeinput', finish);
      pendingConversions.delete(cleanup);
    };
    const finish = (received: Event): void => {
      cleanup();
      if (received !== event) return;
      if (disposed || editor.surface !== surface || JSON.stringify(surface.storyScope()) !== JSON.stringify(scope)) return;
      surface.flushPendingInput();
      const current = surface.state().selection;
      if (current.anchor.paragraphId !== paragraphId || current.head.paragraphId !== paragraphId
        || current.anchor.offset !== offset + 1 || current.head.offset !== offset + 1 || surface.isListParagraph()) return;
      const currentPart = surface.session.partFor(scope);
      if (!currentPart || paragraphTextOf(currentPart, paragraphId) !== before.slice(0, offset) + ' ' + before.slice(offset)) return;
      const prefix = rule.prefix + ' ';
      const match = editor.findMatches(prefix, { matchCase: true, wholeWord: false })
        .find((candidate) => candidate.blockId === paragraphId && candidate.start === 0 && candidate.length === prefix.length);
      if (!match || !editor.can({ type: 'replaceMatch', match, text: '' }).ok) return;
      const command = headingStyle ? { type: 'setParagraphStyle' as const, styleId: headingStyle.styleId }
        : { type: 'toggleList' as const, kind: rule.kind as 'bullet' | 'ordered' };
      if (!editor.can(command).ok) return;
      const after = { paragraphId, start: 0, end: 0 };
      let converted = false;
      const rejection = surface.commitReviewOps((historyToken) => {
        const ops: TreeDocOp[] = [{ op: 'deleteText', paragraphId, start: 0, end: prefix.length }];
        const definition = headingStyle ? null : ensureListDefinition(surface.session.currentPackage(), rule.kind as 'bullet' | 'ordered');
        if (headingStyle) {
          ops.push({ op: 'setParagraphProperties', paragraphId, properties: mergedProperties(directParagraphProperties(currentPart, paragraphId), { localName: 'pStyle', attributes: { val: headingStyle.styleId } }) });
          const size = headingStyle.preview.fontSizePt ?? headingFontSize(headingStyle.styleId);
          if (size) {
            const properties = [{ localName: 'sz', attributes: { val: String(Math.round(size * 2)) } }];
            ops.push({ op: 'setParagraphMarkProperties', paragraphId, properties: mergedParagraphMarkProperties(currentPart, paragraphId, properties) });
            const remaining = before.length - offset;
            if (remaining > 0) {
              for (const edit of runPropertyEdits(currentPart, paragraphId, prefix.length, prefix.length + remaining, properties)) {
                ops.push({ op: 'setRunProperties', paragraphId, start: edit.start - prefix.length, end: edit.end - prefix.length,
                  properties: edit.properties, ...(edit.targetRunIds ? { targetRunIds: edit.targetRunIds } : {}) });
              }
            }
          }
        }
        else if (definition) ops.push({ op: 'setListNumbering', paragraphId, numId: definition.numId, level: 0 });
        else return { committed: false };
        const result = surface.session.applyTreeOps(ops, { paragraphId, start: prefix.length, end: prefix.length }, after, scope,
          { historyToken, packageEdits: definition ? [() => definition.pkg] : undefined });
        converted = result.committed;
        return result;
      }, 'package-scoped');
      if (!rejection && converted) {
        surface.setSelection({ anchor: { paragraphId, offset: 0 }, head: { paragraphId, offset: 0 } });
        reversibleConversion = { paragraphId, scope: JSON.stringify(scope), revision: surface.session.packageRevision() };
      }
    };
    pendingConversions.add(cleanup);
    target.addEventListener('beforeinput', finish, { once: true });
  };
  host.addEventListener('beforeinput', beforeInput, true);
  host.addEventListener('keydown', onKeyDown, true);
  const uninstallEmptyListExit = installEmptyListExit(editor);
  host.ownerDocument.addEventListener('pointerdown', clearReversibleConversion, true);
  return () => {
    disposed = true;
    clearReversibleConversion();
    host.removeEventListener('keydown', onKeyDown, true);
    host.ownerDocument.removeEventListener('pointerdown', clearReversibleConversion, true);
    uninstallHeadingContinuation();
    uninstallEmptyListExit();
    host.removeEventListener('beforeinput', beforeInput, true);
    for (const cleanup of pendingConversions) cleanup();
  };
}
