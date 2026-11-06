import { useEffect, useId, useRef, useState } from 'react';
import { Icon } from './Icon';
import { useDocxEditor, useParagraphStyle } from '@docx-editor.dev/react';
import { paragraphTextOf } from '@docx-editor.dev/core/store';
import { headingFontSize } from './paragraph-styles';

export function ParagraphStylePicker() {
  const editor = useDocxEditor();
  const styles = useParagraphStyle();
  const [open, setOpen] = useState(false);
  const host = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const listId = useId();

  useEffect(() => {
    if (!open) return;
    const dismiss = (event: PointerEvent): void => {
      if (event.target instanceof Node && !host.current?.contains(event.target)) setOpen(false);
    };
    document.addEventListener('pointerdown', dismiss);
    const selected = host.current?.querySelector<HTMLElement>('[aria-selected="true"]');
    (selected ?? host.current?.querySelector<HTMLElement>('[role="option"]'))?.focus();
    return () => document.removeEventListener('pointerdown', dismiss);
  }, [open]);

  function applyStyle(styleId: string): void {
    const surface = editor?.surface;
    if (!editor || !surface) return;
    const selection = surface.state().selection;
    const scope = editor.getActiveScope();
    if (scope.kind !== 'body' && scope.kind !== 'headerFooter') { styles.setValue(styleId); return; }
    const part = surface.session.partFor(scope);
    if (!part) return;
    const paragraphs = surface.session.paragraphIdsIn(scope);
    const anchorIndex = paragraphs.indexOf(selection.anchor.paragraphId);
    const headIndex = paragraphs.indexOf(selection.head.paragraphId);
    if (anchorIndex < 0 || headIndex < 0) return;
    const first = paragraphs[Math.min(anchorIndex, headIndex)];
    const last = paragraphs[Math.max(anchorIndex, headIndex)];
    const size = styles.options.find((style) => style.styleId === styleId)?.preview.fontSizePt ?? headingFontSize(styleId);
    const group = editor.beginHistoryGroup();
    try {
      surface.setSelection({ anchor: { paragraphId: first, offset: 0 }, head: { paragraphId: last, offset: paragraphTextOf(part, last)?.length ?? 0 } });
      const result = editor.exec({ type: 'setParagraphStyle', styleId }, { historyGroup: group });
      if (result.ok && size) editor.exec({ type: 'setMarkAttr', mark: 'fontSize', attr: 'val', value: Math.round(size * 2) }, { historyGroup: group });
    } finally {
      surface.setSelection(selection);
      group.end();
    }
    editor.focus();
  }

  const selectedStyle = styles.options.find((style) => style.styleId === styles.value);
  const labelFor = (name: string): string => name.replace(/\b\p{L}/gu, (letter) => letter.toUpperCase());

  return <div ref={host} className="paragraph-style-picker" onBlur={(event) => {
    if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
  }} onKeyDown={(event) => {
    if (event.key === 'Escape' || event.key === 'Tab') {
      setOpen(false);
      if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); trigger.current?.focus(); }
      return;
    }
    const options = Array.from(host.current?.querySelectorAll<HTMLButtonElement>('[role="option"]') ?? []);
    const index = options.indexOf(document.activeElement as HTMLButtonElement);
    let next = index;
    if (event.key === 'ArrowDown') next = (index + 1) % options.length;
    else if (event.key === 'ArrowUp') next = (index - 1 + options.length) % options.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = options.length - 1;
    else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
      next = options.findIndex((option, optionIndex) => optionIndex > index && option.textContent?.toLowerCase().startsWith(event.key.toLowerCase()));
      if (next < 0) next = options.findIndex((option) => option.textContent?.toLowerCase().startsWith(event.key.toLowerCase()));
    }
    else return;
    if (!open && ['ArrowDown', 'ArrowUp'].includes(event.key)) { event.preventDefault(); setOpen(true); return; }
    if (open && next >= 0) { event.preventDefault(); options[next]?.focus(); }
  }}>
    <button ref={trigger} type="button" role="combobox" aria-label="Paragraph Style" aria-haspopup="listbox" aria-expanded={open} aria-controls={open ? listId : undefined}
      className="docx-toolbar__style-trigger" disabled={!styles.isEnabled} onMouseDown={(event) => event.preventDefault()} onClick={() => setOpen((value) => !value)}>
      <span>{selectedStyle ? labelFor(selectedStyle.name) : 'Paragraph Style'}</span>
    </button>
    {open && <div id={listId} role="listbox" aria-label="Paragraph Styles" className="docx-toolbar__menu paragraph-style-menu">
      <div className="docx-toolbar__menu-label">Paragraph Style</div>
      {styles.options.filter((style) => !/^heading\s*[6-9]$/i.test(style.styleId)).map((style) => <button type="button" role="option" tabIndex={-1} aria-selected={styles.value === style.styleId}
        className="docx-toolbar__menu-item" key={style.styleId} onMouseDown={(event) => event.preventDefault()}
        onClick={() => { setOpen(false); applyStyle(style.styleId); }}>
        <span>{labelFor(style.name)}</span>{styles.value === style.styleId && <Icon name="check" size={14}/>}
      </button>)}
    </div>}
  </div>;
}
