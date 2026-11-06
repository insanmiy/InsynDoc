import { parseSpellingCorrections, type SpellingCorrection } from './spelling-learning';

export type Theme = 'light' | 'dark' | 'system';

export interface Preferences {
  theme: Theme;
  showRuler: boolean;
  showOutline: boolean;
  showWordCount: boolean;
  rulerUnit: 'inch' | 'cm';
  wordGoal: number;
  browserPath: string | null;
  checkSpelling: boolean;
  spellingWords: string[];
  spellingCorrections: SpellingCorrection[];
}

export const PREFERENCES_KEY = 'insyndoc.preferences.v1';
export const DEFAULT_PREFERENCES: Preferences = {
  theme: 'light', showRuler: true, showOutline: true, showWordCount: true, rulerUnit: 'inch', wordGoal: 0, browserPath: null, checkSpelling: true, spellingWords: [], spellingCorrections: []
};

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
      spellingCorrections: parseSpellingCorrections(settings.spellingCorrections),
      spellingWords: Array.isArray(settings.spellingWords) ? [...new Set(settings.spellingWords.filter((word): word is string => typeof word === 'string' && word.length > 0 && word.length <= 64 && /^[\p{L}'\u2019]+$/u.test(word)))].slice(0, 5000) : [],
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
