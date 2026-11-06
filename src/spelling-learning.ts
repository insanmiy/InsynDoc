import { isSpellingWord, spellingKey } from './spelling-engine';

export interface SpellingCorrection {
  source: string;
  replacement: string;
  count: number;
}

export function parseSpellingCorrections(value: unknown): SpellingCorrection[] {
  if (!Array.isArray(value)) return [];
  const corrections = new Map<string, SpellingCorrection>();
  for (const entry of value.slice(0, 1000)) {
    if (!entry || typeof entry !== 'object') continue;
    const { source, replacement, count } = entry as Record<string, unknown>;
    if (!isSpellingWord(source) || !isSpellingWord(replacement) || typeof count !== 'number' || !Number.isInteger(count) || count < 1 || count > 10000) continue;
    const normalized = spellingKey(source);
    if (normalized === spellingKey(replacement)) continue;
    const key = JSON.stringify([normalized, spellingKey(replacement)]);
    const existing = corrections.get(key);
    corrections.set(key, { source: normalized, replacement: existing?.replacement ?? replacement, count: Math.min(10000, count + (existing?.count ?? 0)) });
  }
  return [...corrections.values()];
}

export function learnSpellingCorrection(history: readonly SpellingCorrection[], source: string, replacement: string): SpellingCorrection[] {
  if (!isSpellingWord(source) || !isSpellingWord(replacement) || spellingKey(source) === spellingKey(replacement)) return [...history];
  const normalized = spellingKey(source);
  const existing = history.find((entry) => entry.source === normalized && spellingKey(entry.replacement) === spellingKey(replacement));
  return [
    { source: normalized, replacement, count: Math.min(10000, (existing?.count ?? 0) + 1) },
    ...history.filter((entry) => entry !== existing)
  ].slice(0, 1000);
}
