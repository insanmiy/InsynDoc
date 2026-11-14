import { useEffect, useState } from 'react';
import type { SnapshotInfo, TemplateInfo } from './bridge';
import type { DocumentSession } from './document-session';
import { BUILTIN_TEMPLATES, builtinTemplateBytes, templateDisplayName } from './document-templates';
import { Icon } from './Icon';
import { Modal } from './Modal';

function formatSnapshotTime(createdAt: number): string {
  return new Date(createdAt).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

export function TemplateGallery({ session, onClose }: { session: DocumentSession; onClose: () => void }) {
  const [saved, setSaved] = useState<TemplateInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [templateName, setTemplateName] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let disposed = false;
    session.listTemplates().then((templates) => { if (!disposed) setSaved(templates); })
      .catch((reason: unknown) => { if (!disposed) setError(reason instanceof Error ? reason.message : String(reason)); });
    return () => { disposed = true; };
  }, [session]);

  async function refreshSaved(): Promise<void> {
    try { setSaved(await session.listTemplates()); setError(null); }
    catch (reason: unknown) { setError(reason instanceof Error ? reason.message : String(reason)); }
  }

  async function openSaved(template: TemplateInfo): Promise<void> {
    try {
      const bytes = await session.readTemplate(template.id);
      onClose();
      void session.openTemplate(templateDisplayName(template.name), bytes);
    } catch (reason: unknown) { setError(reason instanceof Error ? reason.message : String(reason)); }
  }

  async function saveCurrent(): Promise<void> {
    const name = templateName.trim();
    if (!name || saving) return;
    setSaving(true);
    try {
      await session.saveAsTemplate(name);
      setTemplateName('');
      await refreshSaved();
    } catch (reason: unknown) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setSaving(false); }
  }

  return <Modal open onClose={onClose} title="Document templates" className="templates-modal">
    <p className="modal-description">Start a new document from a template. Templates stay on this device.</p>
    {error && <p className="form-error" role="alert">{error}</p>}
    <div className="template-grid">
      {BUILTIN_TEMPLATES.map((template) => <button key={template.id} className="template-card" onClick={() => {
        onClose();
        void session.openTemplate(templateDisplayName(template.name), builtinTemplateBytes(template));
      }}>
        <Icon name="document" size={22}/>
        <strong>{template.name}</strong>
        <span>{template.description}</span>
      </button>)}
      {saved?.map((template) => <button key={template.id} className="template-card" onClick={() => void openSaved(template)}>
        <Icon name="document" size={22}/>
        <strong>{template.name}</strong>
        <span>Saved {formatSnapshotTime(template.createdAt)}</span>
      </button>)}
    </div>
    <div className="template-save-row">
      <input aria-label="Template name" placeholder="Template name" value={templateName} onChange={(event) => setTemplateName(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); void saveCurrent(); } }}/>
      <button className="secondary-button" disabled={!templateName.trim() || saving} onClick={() => void saveCurrent()}>{saving ? 'Saving...' : 'Save current document as template'}</button>
    </div>
    <footer className="modal-footer"><button className="primary-button" onClick={onClose}>Done</button></footer>
  </Modal>;
}

export function VersionHistory({ session, onClose }: { session: DocumentSession; onClose: () => void }) {
  const [snapshots, setSnapshots] = useState<SnapshotInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [restoring, setRestoring] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    session.listSnapshots().then((entries) => { if (!disposed) setSnapshots(entries); })
      .catch((reason: unknown) => { if (!disposed) setError(reason instanceof Error ? reason.message : String(reason)); });
    return () => { disposed = true; };
  }, [session]);

  async function restore(snapshot: SnapshotInfo): Promise<void> {
    if (restoring) return;
    setRestoring(snapshot.id);
    setError(null);
    try {
      await session.restoreSnapshot(snapshot.id);
      onClose();
    } catch (reason: unknown) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setRestoring(null); }
  }

  return <Modal open onClose={onClose} title="Version history" className="history-modal">
    <p className="modal-description">InsynDoc saves local snapshots while you work. Restoring replaces the current document.</p>
    {error && <p className="form-error" role="alert">{error}</p>}
    <div className="history-list">
      {snapshots === null && <p className="modal-description">Loading snapshots...</p>}
      {snapshots?.length === 0 && <p className="modal-description">No snapshots yet. Snapshots are saved automatically every two minutes while the document has unsaved changes.</p>}
      {snapshots?.map((snapshot) => <div key={snapshot.id} className="history-row">
        <Icon name="document" size={18}/>
        <span>{formatSnapshotTime(snapshot.createdAt)}</span>
        <button className="secondary-button" disabled={restoring !== null} onClick={() => void restore(snapshot)}>{restoring === snapshot.id ? 'Restoring...' : 'Restore'}</button>
      </div>)}
    </div>
    <footer className="modal-footer"><button className="primary-button" onClick={onClose}>Done</button></footer>
  </Modal>;
}
