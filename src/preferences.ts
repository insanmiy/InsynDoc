import { parseSpellingCorrections, type SpellingCorrection } from './spelling-learning';

export type Theme = 'light' | 'dark' | 'system';

export interface SnippetExpansion {
  trigger: string;
  text: string;
}

export interface Preferences {
  theme: Theme;
  showRuler: boolean;
  showOutline: boolean;
  showWordCount: boolean;
  rulerUnit: 'inch' | 'cm';
  wordGoal: number;
  browserPath: string | null;
  checkSpelling: boolean;
  updateNotifications: boolean;
  spellingWords: string[];
  spellingCorrections: SpellingCorrection[];
  smartPunctuation: boolean;
  snippetExpansions: SnippetExpansion[];
  sanitizeMetadataOnExport: boolean;
  autoRecovery: boolean;
  localFonts: boolean;
}

export const PREFERENCES_KEY = 'insyndoc.preferences.v1';
export const DEFAULT_PREFERENCES: Preferences = {
  theme: 'light', showRuler: true, showOutline: true, showWordCount: true, rulerUnit: 'inch', wordGoal: 0, browserPath: null, checkSpelling: true, updateNotifications: true, spellingWords: [], spellingCorrections: [],
  smartPunctuation: true, snippetExpansions: [], sanitizeMetadataOnExport: true, autoRecovery: true, localFonts: true
};

export function parseSnippetExpansions(value: unknown): SnippetExpansion[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const expansions: SnippetExpansion[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue;
    const trigger = (entry as Record<string, unknown>).trigger;
    const text = (entry as Record<string, unknown>).text;
    if (typeof trigger !== 'string' || typeof text !== 'string') continue;
    const cleanTrigger = trigger.trim();
    const cleanText = text.replace(/\r\n?/g, ' ').trim();
    if (!cleanTrigger || cleanTrigger.length > 32 || /\s/.test(cleanTrigger) || !/^[!-~]+$/.test(cleanTrigger)) continue;
    if (!cleanText || cleanText.length > 2000 || seen.has(cleanTrigger.toLowerCase())) continue;
    seen.add(cleanTrigger.toLowerCase());
    expansions.push({ trigger: cleanTrigger, text: cleanText });
    if (expansions.length >= 100) break;
  }
  return expansions;
}

export function parsePreferences(value: string | null): Preferences {
  if (!value) return { ...DEFAULT_PREFERENCES };
  try {
    const parsed: unknown = JSON.parse(value);
    if (!parsed || typeof parsed !== 'object') return { ...DEFAULT_PREFERENCES };
    const settings = parsed as Record<string, unknown>;
    return {
      theme: ['light', 'dark', 'system'].includes(String(settings.theme)) ? settings.theme as Theme : 'light',
      showRuler: typeof settings.showRuler === 'boolean' ? settings.showRuler : true,
      showOutline: typeof settings.showOutline === 'boolean' ? settings.showOutline : true,
      showWordCount: typeof settings.showWordCount === 'boolean' ? settings.showWordCount : true,
      rulerUnit: settings.rulerUnit === 'cm' ? 'cm' : 'inch',
      checkSpelling: typeof settings.checkSpelling === 'boolean' ? settings.checkSpelling : true,
      updateNotifications: typeof settings.updateNotifications === 'boolean' ? settings.updateNotifications : true,
      smartPunctuation: typeof settings.smartPunctuation === 'boolean' ? settings.smartPunctuation : true,
      sanitizeMetadataOnExport: typeof settings.sanitizeMetadataOnExport === 'boolean' ? settings.sanitizeMetadataOnExport : true,
      autoRecovery: typeof settings.autoRecovery === 'boolean' ? settings.autoRecovery : true,
      localFonts: typeof settings.localFonts === 'boolean' ? settings.localFonts : true,
      snippetExpansions: parseSnippetExpansions(settings.snippetExpansions),
      spellingCorrections: parseSpellingCorrections(settings.spellingCorrections),
      spellingWords: Array.isArray(settings.spellingWords) ? [...new Set(settings.spellingWords.filter((word): word is string => typeof word === 'string' && word.length > 0 && word.length <= 64 && /^[\p{L}'’]+$/u.test(word)))].slice(0, 5000) : [],
      browserPath: typeof settings.browserPath === 'string' && settings.browserPath.trim() ? settings.browserPath : null,
      wordGoal: typeof settings.wordGoal === 'number' && Number.isInteger(settings.wordGoal) && settings.wordGoal >= 0 && settings.wordGoal <= 100000 ? settings.wordGoal : 0
    };
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

export function readPreferences(): Preferences {
  try { return parsePreferences(localStorage.getItem(PREFERENCES_KEY)); }
  catch { return { ...DEFAULT_PREFERENCES }; }
}

export function storePreferences(settings: Preferences): void {
  try { localStorage.setItem(PREFERENCES_KEY, JSON.stringify(settings)); }
  catch { return; }
}
