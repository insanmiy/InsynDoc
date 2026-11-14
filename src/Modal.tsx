import { useLayoutEffect, useRef, type ReactNode } from 'react';
import { Icon } from './Icon';

export function Modal({ open, title, onClose, children, className = '' }: {
  open: boolean; title: string; onClose: () => void; children: ReactNode; className?: string;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    const dialog = ref.current;
    if (!open || !dialog) return;
    if (!dialog.open) dialog.showModal();
    return () => { if (dialog.open) dialog.close(); };
  }, [open]);
  return <dialog ref={ref} className={`modal ${className}`} aria-label={title}
    onCancel={(event) => { event.preventDefault(); onClose(); }}
    onClick={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <div className="modal-content">
      <header className="modal-header"><h2>{title}</h2><button className="icon-button" onClick={onClose} aria-label={`Close ${title}`}><Icon name="close" /></button></header>
      {children}
    </div>
  </dialog>;
}
