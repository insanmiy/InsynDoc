import { useState } from 'react';
import { chooseLinkBrowser } from './bridge';
import { Modal } from './Modal';
import { Icon, type IconName } from './Icon';
import { parseSnippetExpansions, type Preferences, type SnippetExpansion, type Theme } from './preferences';
import type { UpdateCheckResult } from './updater';

const themes: { value: Theme; label: string; icon: IconName }[] = [
  { value: 'light', label: 'Light', icon: 'sun' },
  { value: 'dark', label: 'Dark', icon: 'moon' },
  { value: 'system', label: 'System', icon: 'monitor' }
];

type UpdateStatus = UpdateCheckResult['status'] | 'idle' | 'checking';

export function Settings({ open, onClose, preferences, onChange, appVersion, onCheckForUpdates }: {
  open: boolean; onClose: () => void; preferences: Preferences; onChange: (settings: Preferences) => void;
  appVersion: string; onCheckForUpdates: () => Promise<UpdateCheckResult>;
}) {
  const [browserError, setBrowserError] = useState<string | null>(null);
  const [choosingBrowser, setChoosingBrowser] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('idle');
  const [snippetTrigger, setSnippetTrigger] = useState('');
  const [snippetText, setSnippetText] = useState('');
  const [snippetError, setSnippetError] = useState<string | null>(null);

  function addSnippet(): void {
    const expansions = parseSnippetExpansions([...preferences.snippetExpansions, { trigger: snippetTrigger, text: snippetText }]);
    if (expansions.length === preferences.snippetExpansions.length) {
      setSnippetError('Use A Short Trigger Without Spaces And Replacement Text Up To 2,000 Characters.');
      return;
    }
    onChange({ ...preferences, snippetExpansions: expansions });
    setSnippetTrigger('');
    setSnippetText('');
    setSnippetError(null);
  }

  function removeSnippet(trigger: string): void {
    onChange({ ...preferences, snippetExpansions: preferences.snippetExpansions.filter((snippet) => snippet.trigger !== trigger) });
  }

  async function checkUpdates(): Promise<void> {
    setUpdateStatus('checking');
    try {
      setUpdateStatus((await onCheckForUpdates()).status);
    } catch {
      setUpdateStatus('error');
    }
  }

  async function chooseBrowser(): Promise<void> {
    setBrowserError(null);
    setChoosingBrowser(true);
    try {
      const browserPath = await chooseLinkBrowser();
      if (browserPath) onChange({ ...preferences, browserPath });
    } catch (error: unknown) {
      setBrowserError(error instanceof Error ? error.message : String(error));
    } finally { setChoosingBrowser(false); }
  }

  return <Modal open={open} title="Settings" onClose={onClose} className="settings-modal">
    <p className="modal-description settings-intro">Personalize Your Writing Workspace. Changes Are Saved Automatically On This Device.</p>
    <nav className="settings-navigation" aria-label="Settings Sections">
      {[
        ['appearance-heading', 'Appearance'],
        ['writing-heading', 'Writing'],
        ['editor-heading', 'Editor'],
        ['updates-heading', 'Updates'],
        ['links-heading', 'Hyperlinks']
      ].map(([id, label]) => <button type="button" key={id} onClick={() => document.getElementById(id)?.scrollIntoView({ block: 'start' })}>{label}</button>)}
    </nav>
    <section className="settings-section" aria-labelledby="appearance-heading">
      <h3 id="appearance-heading">Appearance</h3><p>Choose A Theme For The App. Your Document Colors Stay The Same.</p>
      <div className="theme-options" role="group" aria-label="App Theme">
        {themes.map((theme) => <button key={theme.value} className={`theme-option theme-preview-${theme.value}`}
          type="button" aria-label={theme.label} title={theme.value === 'system' ? 'Follow Your Device Theme' : `Use ${theme.label} Theme`}
          aria-pressed={preferences.theme === theme.value} onClick={() => onChange({ ...preferences, theme: theme.value })}>
          <span className="theme-preview" aria-hidden="true"><span className="preview-toolbar"/><span className="preview-sidebar"/><span className="preview-page"><i/><i/><i/></span></span>
          <span className="theme-caption"><Icon name={theme.icon} size={17}/>{theme.label}{preferences.theme === theme.value && <Icon name="check" size={17}/>}</span>
        </button>)}
      </div>
    </section>
    <section className="settings-section" aria-labelledby="writing-heading">
      <h3 id="writing-heading">Writing Preferences</h3>
      <p>Set Your Measurement Units And Writing Target.</p>
      <label className="setting-field"><span>Ruler Units</span><select value={preferences.rulerUnit} onChange={(event) => onChange({ ...preferences, rulerUnit: event.target.value === 'cm' ? 'cm' : 'inch' })}><option value="inch">Inches</option><option value="cm">Centimeters</option></select></label>
      <label className="setting-field"><span>Word Goal<small id="word-goal-description">Set To 0 To Turn Off. Up To 100,000 Words.</small></span><input aria-label="Word Goal" aria-describedby="word-goal-description" type="number" min="0" max="100000" step="1" value={preferences.wordGoal} onChange={(event) => {
        const goal = Number(event.target.value);
        if (Number.isInteger(goal) && goal >= 0 && goal <= 100000) onChange({ ...preferences, wordGoal: goal });
      }}/></label>
      <h4 className="settings-subheading">Text Expansions</h4>
      <p>Type A Trigger Followed By A Space To Insert Replacement Text. For Example, ;sig.</p>
      {preferences.snippetExpansions.map((snippet: SnippetExpansion) => <div className="snippet-row" key={snippet.trigger}>
        <code>{snippet.trigger}</code><span>{snippet.text}</span>
        <button className="icon-button" aria-label={`Remove ${snippet.trigger}`} onClick={() => removeSnippet(snippet.trigger)}><Icon name="close" size={15}/></button>
      </div>)}
      <div className="snippet-form">
        <input aria-label="Snippet Trigger" placeholder="Trigger, Like ;sig" value={snippetTrigger} onChange={(event) => { setSnippetTrigger(event.target.value); setSnippetError(null); }}/>
        <input aria-label="Snippet Text" placeholder="Replacement Text" value={snippetText} onChange={(event) => { setSnippetText(event.target.value); setSnippetError(null); }} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); addSnippet(); } }}/>
        <button className="secondary-button" disabled={!snippetTrigger.trim() || !snippetText.trim()} onClick={addSnippet}>Add</button>
      </div>
      {snippetError && <p className="form-error" role="alert">{snippetError}</p>}
    </section>
    <section className="settings-section" aria-labelledby="editor-heading">
      <h3 id="editor-heading">Editor Preferences</h3>
      <p>Choose The Tools Visible In Your Workspace.</p>
      {([
        ['checkSpelling', 'Check Spelling', 'Check English (US) Spelling And Sentence Context On This Device.'],
        ['showRuler', 'Show Ruler', 'Adjust Page Margins And Indents.'],
        ['showOutline', 'Show Document Outline', 'Navigate Between Your Headings.'],
        ['showWordCount', 'Show Word Count', 'Keep Document Statistics In The Status Bar.'],
        ['smartPunctuation', 'Smart Punctuation', 'Convert Straight Quotes To Curly Quotes And Double Dashes To Em Dashes While Typing.'],
        ['sanitizeMetadataOnExport', 'Remove Hidden Metadata On Export', 'Strip Author, Company, Timestamps, And Revision Details From Downloaded Or Exported Copies.'],
        ['autoRecovery', 'Auto Recovery Snapshots', 'Save Local Snapshots Every Two Minutes While The Document Has Unsaved Changes.'],
        ['localFonts', 'Local Font Discovery', 'Use Fonts Installed On This Device So Documents Render With Their Intended Typefaces Offline.']
      ] as const).map(([key, label, description]) => <label className="setting-row" key={key}>
        <span><strong id={`${key}-label`}>{label}</strong><small id={`${key}-description`}>{description}</small></span>
        <input type="checkbox" role="switch" aria-labelledby={`${key}-label`} aria-describedby={`${key}-description`} checked={preferences[key]} onChange={(event) => onChange({ ...preferences, [key]: event.target.checked })}/>
      </label>)}
      {preferences.spellingWords.length > 0 && <div className="personal-dictionary-setting"><span>{preferences.spellingWords.length} {preferences.spellingWords.length === 1 ? 'Word' : 'Words'} In Your Personal Dictionary</span><button className="text-button" onClick={() => onChange({ ...preferences, spellingWords: [] })}>Reset Personal Dictionary</button></div>}
      {preferences.spellingCorrections.length > 0 && <div className="personal-dictionary-setting"><span>Suggestions Learn From Corrections You Accept On This Device.</span><button className="text-button" onClick={() => onChange({ ...preferences, spellingCorrections: [] })}>Reset Learned Suggestions</button></div>}
    </section>
    <section className="settings-section" aria-labelledby="updates-heading">
      <h3 id="updates-heading">Updates</h3>
      <p>Keep InsynDoc On The Latest Version.</p>
      <label className="setting-row">
        <span><strong id="update-notifications-label">Notify Me About Updates</strong><small id="update-notifications-description">Show A Prompt When A New Version Is Available.</small></span>
        <input type="checkbox" role="switch" aria-labelledby="update-notifications-label" aria-describedby="update-notifications-description" checked={preferences.updateNotifications} onChange={(event) => onChange({ ...preferences, updateNotifications: event.target.checked })}/>
      </label>
      <div className="update-check">
        <div className="update-check-details">
          <span className="update-version">Version {appVersion}</span>
          {updateStatus === 'current' && <p className="modal-description" role="status">You're Up To Date.</p>}
          {updateStatus === 'available' && <p className="modal-description" role="status">A Newer Version Is Available. Finish Installing From The Update Prompt.</p>}
          {updateStatus === 'error' && <p className="form-error" role="alert">Could Not Check For Updates. Check Your Connection And Try Again.</p>}
          {updateStatus === 'unsupported' && <p className="modal-description" role="status">Update Checks Work In The Installed App.</p>}
        </div>
        <button className="secondary-button" disabled={updateStatus === 'checking'} onClick={() => void checkUpdates()}>{updateStatus === 'checking' ? 'Checking...' : 'Check For Updates'}</button>
      </div>
    </section>
    <section className="settings-section" aria-labelledby="links-heading">
      <h3 id="links-heading">Hyperlinks</h3><p>Open Links With Your System Default Or A Browser You Choose.</p>
      <label className="setting-field"><span>Link Browser</span><select aria-label="Link Browser" disabled={choosingBrowser} value={preferences.browserPath ? 'custom' : 'system'} onChange={(event) => {
        if (event.target.value === 'system') { setBrowserError(null); onChange({ ...preferences, browserPath: null }); }
        else void chooseBrowser();
      }}><option value="system">System Default</option><option value="custom">Custom Browser</option></select></label>
      {preferences.browserPath && <p className="browser-path" title={preferences.browserPath}>{preferences.browserPath}</p>}
      <div className="browser-actions"><button className="secondary-button" disabled={choosingBrowser} onClick={() => void chooseBrowser()}>{choosingBrowser ? 'Choosing Browser...' : 'Choose Browser'}</button></div>
      {browserError && <p className="form-error" role="alert">{browserError}</p>}
    </section>
    <footer className="modal-footer"><button className="primary-button" onClick={onClose}>Done</button></footer>
  </Modal>;
}
