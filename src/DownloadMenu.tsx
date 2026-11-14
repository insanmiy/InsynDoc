import { useEffect, useRef, useState } from 'react';
import type { ExportFormat } from './document-export';
import { Icon } from './Icon';

const formats: readonly { format: ExportFormat; label: string }[] = [
  { format: 'docx', label: 'Microsoft Word (.docx)' },
  { format: 'pdf', label: 'PDF document (.pdf)' },
  { format: 'odt', label: 'OpenDocument format (.odt)' },
  { format: 'txt', label: 'Plain text (.txt)' },
  { format: 'rtf', label: 'Rich text format (.rtf)' },
  { format: 'html', label: 'Web page (.html, zipped)' },
  { format: 'epub', label: 'EPUB publication (.epub)' },
  { format: 'md', label: 'Markdown (.md)' }
];

export function DownloadMenu({ disabled, onDownload }: { disabled: boolean; onDownload: (format: ExportFormat) => Promise<void> }) {
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const openAtEnd = useRef(false);

  async function download(format: ExportFormat): Promise<void> {
    trigger.current?.focus();
    setOpen(false);
    try { await onDownload(format); }
    finally {
      requestAnimationFrame(() => {
        if (document.activeElement === document.body && !document.querySelector('dialog[open]')) trigger.current?.focus();
      });
    }
  }

  useEffect(() => { if (disabled) setOpen(false); }, [disabled]);

  useEffect(() => {
    if (!open) return;
    const buttons = menu.current?.querySelectorAll<HTMLButtonElement>('button');
    buttons?.[openAtEnd.current ? buttons.length - 1 : 0]?.focus();
    const outside = (event: PointerEvent): void => {
      if (event.target instanceof Node && !container.current?.contains(event.target)) setOpen(false);
    };
    document.addEventListener('pointerdown', outside, true);
    return () => document.removeEventListener('pointerdown', outside, true);
  }, [open]);

  return <div ref={container} className="download-menu-container" onBlur={(event) => {
    if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
  }}>
    <button ref={trigger} className="primary-button download-document" aria-label="Download document" aria-haspopup="menu" aria-expanded={open} disabled={disabled} onClick={() => { openAtEnd.current = false; setOpen((value) => !value); }} onKeyDown={(event) => {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); openAtEnd.current = event.key === 'ArrowUp'; setOpen(true); }
    }}><Icon name="download" size={16}/>Download<Icon name="chevronDown" size={14}/></button>
    {open && <div ref={menu} className="download-format-menu" role="menu" aria-label="Download formats" onKeyDown={(event) => {
      if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); setOpen(false); trigger.current?.focus(); return; }
      if (event.key === 'Tab') { trigger.current?.focus(); setOpen(false); return; }
      if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
      event.preventDefault();
      const buttons = [...menu.current!.querySelectorAll<HTMLButtonElement>('button')];
      const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (current + (event.key === 'ArrowUp' ? -1 : 1) + buttons.length) % buttons.length;
      buttons[next]?.focus();
    }}>{formats.map(({ format, label }) => <button key={format} role="menuitem" tabIndex={-1} onClick={() => void download(format)}>{label}</button>)}</div>}
  </div>;
}
