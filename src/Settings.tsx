import { useRef, useState } from 'react';
import { chooseLinkBrowser, isDesktopApp } from './bridge';
import { Modal } from './Modal';
import { Icon, type IconName } from './Icon';
import { INTERFACE_STYLES, parseSnippetExpansions, type Preferences, type SnippetExpansion, type Theme } from './preferences';
import type { UpdateCheckResult } from './updater';
import './settings-layouts.css';

const themes: { value: Theme; label: string; icon: IconName }[] = [
  { value: 'light', label: 'Light', icon: 'sun' },
  { value: 'dark', label: 'Dark', icon: 'moon' },
  { value: 'system', label: 'System', icon: 'monitor' }
];

type UpdateStatus = UpdateCheckResult['status'] | 'idle' | 'checking';

const sections = [
  { id: 'appearance', label: 'Appearance', title: 'Appearance', icon: 'monitor', description: 'Choose a theme for the app. Your document colors stay the same.' },
  { id: 'writing', label: 'Writing', title: 'Writing preferences', icon: 'edit', description: 'Set your measurement units and writing target.' },
  { id: 'editor', label: 'Editor', title: 'Editor preferences', icon: 'document', description: 'Choose the tools visible in your workspace.' },
  { id: 'updates', label: 'Updates', title: 'Updates', icon: 'download', description: 'Keep InsynDoc on the latest version.' },
  { id: 'links', label: 'Hyperlinks', title: 'Hyperlinks', icon: 'newDocument', description: 'Open links with your system default or a browser you choose.' },
  { id: 'uninstall', label: 'Uninstall', title: 'Uninstall', icon: 'trash', description: 'InsynDoc will close when the Windows uninstaller opens. Confirm or cancel removal there.' }
] as const satisfies readonly { id: string; label: string; title: string; icon: IconName; description: string }[];

type SettingsSection = typeof sections[number]['id'];

export function Settings({ open, onClose, preferences, onChange, appVersion, onCheckForUpdates, onUninstall, uninstallDisabled }: {
  open: boolean; onClose: () => void; preferences: Preferences; onChange: (settings: Preferences) => void;
  appVersion: string; onCheckForUpdates: () => Promise<UpdateCheckResult>;
  onUninstall: () => void; uninstallDisabled: boolean;
}) {
  const [browserError, setBrowserError] = useState<string | null>(null);
  const [choosingBrowser, setChoosingBrowser] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('idle');
  const [snippetTrigger, setSnippetTrigger] = useState('');
  const [snippetText, setSnippetText] = useState('');
  const [snippetError, setSnippetError] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<SettingsSection>('appearance');
  const panels = useRef<HTMLDivElement>(null);
  const desktopApp = isDesktopApp();
  const visibleSections = sections.filter((section) => desktopApp || section.id !== 'uninstall');

  function syncActiveSection(panel: HTMLDivElement): void {
    const readingLine = panel.getBoundingClientRect().top + 48;
    let currentSection: SettingsSection = 'appearance';
    for (const { id } of visibleSections) {
      const heading = panel.querySelector(`#${id}-heading`);
      if (!heading || heading.getBoundingClientRect().top > readingLine) break;
      currentSection = id;
    }
    setActiveSection(currentSection);
  }

  function navigateTo(section: SettingsSection): void {
    const target = panels.current?.querySelector(`#${section}-settings`);
    target?.querySelector('h3')?.focus({ preventScroll: true });
    target?.scrollIntoView({ block: 'start' });
  }

  function addSnippet(): void {
    const expansions = parseSnippetExpansions([...preferences.snippetExpansions, { trigger: snippetTrigger, text: snippetText }]);
    if (expansions.length === preferences.snippetExpansions.length) {
      setSnippetError('Use a short trigger without spaces and replacement text up to 2,000 characters.');
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

  const sectionContents = {
    appearance: <>
      <div className="theme-options" role="group" aria-label="App theme">
        {themes.map((theme) => <button key={theme.value} className={`theme-option theme-preview-${theme.value}`}
          type="button" aria-label={theme.label} title={theme.value === 'system' ? 'Follow your device theme' : `Use ${theme.label.toLowerCase()} theme`}
          aria-pressed={preferences.theme === theme.value} onClick={() => onChange({ ...preferences, theme: theme.value })}>
          <span className="theme-preview" aria-hidden="true"><span className="preview-toolbar"/><span className="preview-sidebar"/><span className="preview-page"><i/><i/><i/></span></span>
          <span className="theme-caption"><Icon name={theme.icon} size={17}/>{theme.label}{preferences.theme === theme.value && <Icon name="check" size={17}/>}</span>
        </button>)}
      </div>
      <h4 className="settings-subheading">Interface style</h4>
      <p>Choose from {INTERFACE_STYLES.length} workspaces. Each supports light and dark mode. Changes apply immediately and are saved on this device.</p>
      <div className="interface-options" role="group" aria-label="Interface style">
        {INTERFACE_STYLES.map((style) => <button key={style.value} type="button"
          className={`interface-option interface-preview-${style.value}`} aria-label={style.label}
          aria-pressed={preferences.interfaceStyle === style.value}
          onClick={() => onChange({ ...preferences, interfaceStyle: style.value })}>
          <span className="interface-preview" aria-hidden="true"><i/><i/><i/></span>
          <span className="interface-caption"><strong>{style.label}</strong>{preferences.interfaceStyle === style.value && <Icon name="check" size={16}/>}</span>
          <small>{style.description}</small>
        </button>)}
      </div>
    </>,
    writing: <>
      <label className="setting-field"><span>Ruler units</span><select value={preferences.rulerUnit} onChange={(event) => onChange({ ...preferences, rulerUnit: event.target.value === 'cm' ? 'cm' : 'inch' })}><option value="inch">Inches</option><option value="cm">Centimeters</option></select></label>
      <label className="setting-field"><span>Word goal<small id="word-goal-description">Set to 0 to turn off. Up to 100,000 words.</small></span><input aria-label="Word goal" aria-describedby="word-goal-description" type="number" min="0" max="100000" step="1" value={preferences.wordGoal} onChange={(event) => {
        const goal = Number(event.target.value);
        if (Number.isInteger(goal) && goal >= 0 && goal <= 100000) onChange({ ...preferences, wordGoal: goal });
      }}/></label>
      <h4 className="settings-subheading">Text expansions</h4>
      <p>Type a trigger followed by a space to insert replacement text. For example, ;sig.</p>
      {preferences.snippetExpansions.map((snippet: SnippetExpansion) => <div className="snippet-row" key={snippet.trigger}>
        <code>{snippet.trigger}</code><span>{snippet.text}</span>
        <button className="icon-button" aria-label={`Remove ${snippet.trigger}`} onClick={() => removeSnippet(snippet.trigger)}><Icon name="close" size={15}/></button>
      </div>)}
      <div className="snippet-form">
        <input aria-label="Snippet trigger" placeholder="Trigger, like ;sig" value={snippetTrigger} onChange={(event) => { setSnippetTrigger(event.target.value); setSnippetError(null); }}/>
        <input aria-label="Snippet text" placeholder="Replacement text" value={snippetText} onChange={(event) => { setSnippetText(event.target.value); setSnippetError(null); }} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); addSnippet(); } }}/>
        <button className="secondary-button" disabled={!snippetTrigger.trim() || !snippetText.trim()} onClick={addSnippet}>Add</button>
      </div>
      {snippetError && <p className="form-error" role="alert">{snippetError}</p>}
    </>,
    editor: <>
      {([
        ['checkSpelling', 'Check Spelling', 'Check English (US) Spelling As You Type. Run Check Spelling And Grammar From Tools For A Full Review.'],
        ['showRuler', 'Show ruler', 'Adjust page margins and indents.'],
        ['showOutline', 'Show document outline', 'Navigate between your headings.'],
        ['showWordCount', 'Show word count', 'Keep document statistics in the status bar.'],
        ['smartPunctuation', 'Smart punctuation', 'Convert straight quotes to curly quotes and double dashes to em dashes while typing.'],
        ['sanitizeMetadataOnExport', 'Remove hidden metadata on export', 'Strip author, company, timestamps, and revision details from downloaded or exported copies.'],
        ['autoRecovery', 'Auto recovery snapshots', 'Save local snapshots every two minutes while the document has unsaved changes.'],
        ['localFonts', 'Local font discovery', 'Use fonts installed on this device so documents render with their intended typefaces offline.']
      ] as const).map(([key, label, description]) => <label className="setting-row" key={key}>
        <span><strong id={`${key}-label`}>{label}</strong><small id={`${key}-description`}>{description}</small></span>
        <input type="checkbox" role="switch" aria-labelledby={`${key}-label`} aria-describedby={`${key}-description`} checked={preferences[key]} onChange={(event) => onChange({ ...preferences, [key]: event.target.checked })}/>
      </label>)}
      {preferences.spellingWords.length > 0 && <div className="personal-dictionary-setting"><span>{preferences.spellingWords.length} {preferences.spellingWords.length === 1 ? 'word' : 'words'} in your personal dictionary</span><button className="text-button" onClick={() => onChange({ ...preferences, spellingWords: [] })}>Reset personal dictionary</button></div>}
      {preferences.spellingCorrections.length > 0 && <div className="personal-dictionary-setting"><span>Suggestions learn from corrections you accept on this device.</span><button className="text-button" onClick={() => onChange({ ...preferences, spellingCorrections: [] })}>Reset learned suggestions</button></div>}
    </>,
    updates: <>
      <label className="setting-row">
        <span><strong id="update-notifications-label">Notify me about updates</strong><small id="update-notifications-description">Show a prompt when a new version is available.</small></span>
        <input type="checkbox" role="switch" aria-labelledby="update-notifications-label" aria-describedby="update-notifications-description" checked={preferences.updateNotifications} onChange={(event) => onChange({ ...preferences, updateNotifications: event.target.checked })}/>
      </label>
      <div className="update-check">
        <div className="update-check-details">
          <span className="update-version">Version {appVersion}</span>
          {updateStatus === 'current' && <p className="modal-description" role="status">You're up to date.</p>}
          {updateStatus === 'available' && <p className="modal-description" role="status">A newer version is available. Finish installing from the update prompt.</p>}
          {updateStatus === 'error' && <p className="form-error" role="alert">Could not check for updates. Check your connection and try again.</p>}
          {updateStatus === 'unsupported' && <p className="modal-description" role="status">Update checks work in the installed app.</p>}
        </div>
        <button className="secondary-button" disabled={updateStatus === 'checking'} onClick={() => void checkUpdates()}>{updateStatus === 'checking' ? 'Checking...' : 'Check for updates'}</button>
      </div>
    </>,
    links: <>
      <label className="setting-field"><span>Link browser</span><select aria-label="Link browser" disabled={choosingBrowser} value={preferences.browserPath ? 'custom' : 'system'} onChange={(event) => {
        if (event.target.value === 'system') { setBrowserError(null); onChange({ ...preferences, browserPath: null }); }
        else void chooseBrowser();
      }}><option value="system">System default</option><option value="custom">Custom browser</option></select></label>
      {preferences.browserPath && <p className="browser-path" title={preferences.browserPath}>{preferences.browserPath}</p>}
      <div className="browser-actions"><button className="secondary-button" disabled={choosingBrowser} onClick={() => void chooseBrowser()}>{choosingBrowser ? 'Choosing browser...' : 'Choose browser'}</button></div>
      {browserError && <p className="form-error" role="alert">{browserError}</p>}
    </>,
    uninstall: <div className="browser-actions"><button type="button" className="secondary-button" disabled={uninstallDisabled} onClick={onUninstall}>Uninstall InsynDoc</button></div>
  };

  return <Modal open={open} title="Settings" onClose={onClose} className="settings-modal">
    <p className="modal-description settings-intro">Changes are saved automatically.</p>
    <div className="settings-layout-body">
      <nav className="settings-navigation" aria-label="Settings sections">
        {visibleSections.map(({ id, label, icon }) => <button type="button" key={id}
          aria-current={activeSection === id ? 'location' : undefined} aria-controls={`${id}-settings`}
          onClick={() => navigateTo(id)}>
          <Icon name={icon} size={17}/>
          {label}
        </button>)}
      </nav>
      <div className="settings-panels" ref={panels} onScroll={(event) => syncActiveSection(event.currentTarget)}>
        {visibleSections.map(({ id, title, description }) => <section key={id} className="settings-section"
          id={`${id}-settings`} aria-labelledby={`${id}-heading`}>
          <h3 id={`${id}-heading`} tabIndex={-1}>{title}</h3>
          <p>{description}</p>
          {sectionContents[id]}
        </section>)}
      </div>
    </div>
    <footer className="modal-footer"><button className="primary-button" onClick={onClose}>Done</button></footer>
  </Modal>;
}
