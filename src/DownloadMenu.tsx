import { useEffect, useRef, useState } from 'react';
import type { ExportFormat } from './document-export';
import { Icon } from './Icon';

const formats: readonly { format: ExportFormat; label: string }[] = [
  { format: 'docx', label: 'Microsoft Word (.docx)' },
  { format: 'pdf', label: 'PDF Document (.pdf)' },
  { format: 'odt', label: 'OpenDocument Format (.odt)' },
  { format: 'txt', label: 'Plain Text (.txt)' },
  { format: 'rtf', label: 'Rich Text Format (.rtf)' },
  { format: 'html', label: 'Web Page (.html, Zipped)' },
  { format: 'epub', label: 'EPUB Publication (.epub)' },
  { format: 'md', label: 'Markdown (.md)' }
];

export function DownloadMenu({ disabled, onDownload }: { disabled: boolean; onDownload: (format: ExportFormat) => void }) {
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);

  useEffect(() => { if (disabled) setOpen(false); }, [disabled]);

  useEffect(() => {
    if (!open) return;
    menu.current?.querySelector<HTMLButtonElement>('button')?.focus();
    const outside = (event: PointerEvent): void => {
      if (event.target instanceof Node && !container.current?.contains(event.target)) setOpen(false);
    };
    document.addEventListener('pointerdown', outside, true);
    return () => document.removeEventListener('pointerdown', outside, true);
  }, [open]);

  return <div ref={container} className="download-menu-container">
    <button ref={trigger} className="primary-button download-document" aria-label="Download Document" aria-haspopup="menu" aria-expanded={open} disabled={disabled} onClick={() => setOpen((value) => !value)} onKeyDown={(event) => {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); setOpen(true); }
    }}><Icon name="download" size={16}/>Download<Icon name="chevronDown" size={14}/></button>
    {open && <div ref={menu} className="download-format-menu" role="menu" aria-label="Download Formats" onKeyDown={(event) => {
      if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); setOpen(false); trigger.current?.focus(); return; }
      if (event.key === 'Tab') { setOpen(false); return; }
      if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
      event.preventDefault();
      const buttons = [...menu.current!.querySelectorAll<HTMLButtonElement>('button')];
      const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (current + (event.key === 'ArrowUp' ? -1 : 1) + buttons.length) % buttons.length;
      buttons[next]?.focus();
    }}>{formats.map(({ format, label }) => <button key={format} role="menuitem" tabIndex={-1} onClick={() => { setOpen(false); onDownload(format); }}>{label}</button>)}</div>}
  </div>;
}
