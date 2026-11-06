import type { SpellingCorrection } from './spelling-learning';

export interface SpellingParagraph {
  blockId: string;
  text: string;
}

export interface SpellingIssue {
  blockId: string;
  start: number;
  length: number;
  word: string;
  expectedText: string;
  kind?: 'grammar';
  message?: string;
  suggestions?: string[];
  ignoreKey?: string;
}

export type SpellingRequest =
  | { type: 'check'; id: number; paragraphs: SpellingParagraph[]; excluded: string[]; ignoredGrammar?: string[] }
  | { type: 'suggest'; id: number; word: string; personalWords?: string[]; corrections?: SpellingCorrection[] };

export type SpellingResponse =
  | { type: 'checked'; id: number; issues: SpellingIssue[]; truncated: boolean }
  | { type: 'suggested'; id: number; suggestions: string[] }
  | { type: 'error'; id: number };

export function spellingKey(word: string): string {
  return word.replace(/\u2019/g, "'").toLocaleLowerCase('en-US');
}

// Break Equally Close Matches In Favor Of Common English Function Words.
const commonWords = new Set('a an and are as at be been but by can could did do does for from had has have he her here him his how i if in into is it its may me more my no not of on one or our out she should so some than that the their them then there these they this those through to too up us was we were what when where which who why will with would you your'.split(' '));
const spellingWord = /^[\p{L}]+(?:['\u2019][\p{L}]+)*$/u;

export function isSpellingWord(word: unknown): word is string {
  return typeof word === 'string' && word.length <= 64 && spellingWord.test(word);
}

function suggestionDistance(source: string, target: string): number {
  let previousPrevious: number[] = [];
  let previous = Array.from({ length: target.length + 1 }, (_, index) => index);
  for (let sourceIndex = 1; sourceIndex <= source.length; sourceIndex++) {
    const current = [sourceIndex];
    for (let targetIndex = 1; targetIndex <= target.length; targetIndex++) {
      const insertionCost = target[targetIndex - 1] === "'" ? 0.4 : 1;
      current[targetIndex] = Math.min(
        previous[targetIndex] + 1,
        current[targetIndex - 1] + insertionCost,
        previous[targetIndex - 1] + (source[sourceIndex - 1] === target[targetIndex - 1] ? 0 : 1)
      );
      if (sourceIndex > 1 && targetIndex > 1 && source[sourceIndex - 1] === target[targetIndex - 2] && source[sourceIndex - 2] === target[targetIndex - 1]) {
        current[targetIndex] = Math.min(current[targetIndex], previousPrevious[targetIndex - 2] + 0.75);
      }
    }
    previousPrevious = previous;
    previous = current;
  }
  return previous[target.length];
}

export function suggestSpelling(word: string, checker: { correct: (word: string) => boolean; suggest: (word: string) => string[] }, personalWords: readonly string[] = [], corrections: readonly SpellingCorrection[] = []): string[] {
  if (!isSpellingWord(word)) return [];
  const normalized = word.replace(/\u2019/g, "'");
  const key = spellingKey(word);
  if (checker.correct(normalized) || personalWords.some((personal) => spellingKey(personal) === key)) return [];
  // Keep Expensive Dictionary Generation Bounded, Even For Long Personal Words.
  const candidates = normalized.length <= 32 ? checker.suggest(normalized) : [];
  const personalForms = new Map<string, string>();
  const learnedForms = new Map<string, string>();
  const learnedCounts = new Map<string, number>();
  for (const correction of corrections.slice(0, 1000)) {
    if (correction.source !== key || !isSpellingWord(correction.replacement)) continue;
    candidates.push(correction.replacement);
    learnedForms.set(spellingKey(correction.replacement), correction.replacement);
    learnedCounts.set(spellingKey(correction.replacement), correction.count);
  }
  for (const personal of personalWords.slice(0, 5000)) {
    if (personal.length > 64 || !spellingWord.test(personal) || Math.abs(personal.length - normalized.length) > 2) continue;
    if (suggestionDistance(key, spellingKey(personal)) <= (key.length <= 4 ? 1 : 2)) {
      candidates.push(personal);
      personalForms.set(spellingKey(personal), personal);
    }
  }
  const ranked = candidates.map((candidate, index) => ({
    candidate,
    index,
    learned: learnedCounts.get(spellingKey(candidate)) ?? 0,
    distance: suggestionDistance(key, spellingKey(candidate)),
    common: commonWords.has(spellingKey(candidate))
  })).sort((left, right) => right.learned - left.learned || left.distance - right.distance || Number(right.common) - Number(left.common) || left.index - right.index);
  const suggestions = new Map<string, string>();
  for (const { candidate } of ranked) {
    const personal = personalForms.get(spellingKey(candidate));
    const learned = learnedForms.get(spellingKey(candidate));
    const preserveCase = Boolean(personal) || Boolean(learned && !checker.correct(spellingKey(learned)));
    let replacement = personal ?? learned ?? candidate;
    if (learned && !preserveCase) replacement = spellingKey(replacement);
    if (normalized === normalized.toUpperCase()) replacement = replacement.toUpperCase();
    else if (!preserveCase && /^\p{Lu}\p{Ll}/u.test(normalized)) replacement = replacement[0].toUpperCase() + replacement.slice(1);
    if (word.includes('\u2019')) replacement = replacement.replace(/'/g, '\u2019');
    const replacementKey = spellingKey(replacement);
    if (replacementKey !== key && !suggestions.has(replacementKey)) suggestions.set(replacementKey, replacement);
    if (suggestions.size === 5) break;
  }
  return [...suggestions.values()];
}

export function checkParagraphs(paragraphs: readonly SpellingParagraph[], correct: (word: string) => boolean, excluded: readonly string[]) {
  const ignored = new Set(excluded.map(spellingKey));
  const issues: SpellingIssue[] = [];
  const segmenter = new Intl.Segmenter('en-US', { granularity: 'word' });
  const checked = new Map<string, boolean>();
  for (const paragraph of paragraphs) {
    const links = [...paragraph.text.matchAll(/(?:https?:\/\/|www\.)\S+|[\w.+-]+@[\w.-]+\.[a-z]{2,}/gi)]
      .map((match) => ({ start: match.index, end: match.index + match[0].length }));
    for (const token of segmenter.segment(paragraph.text)) {
      const word = token.segment;
      if (!token.isWordLike || word.length > 64 || !/^[\p{L}]+(?:['\u2019][\p{L}]+)*$/u.test(word)) continue;
      if (links.some((link) => token.index >= link.start && token.index < link.end) || ignored.has(spellingKey(word))) continue;
      const normalized = word.replace(/\u2019/g, "'");
      let valid = checked.get(normalized);
      if (valid === undefined) { valid = correct(normalized); checked.set(normalized, valid); }
      if (valid) continue;
      if (issues.length === 1000) return { issues, truncated: true };
      issues.push({ blockId: paragraph.blockId, start: token.index, length: word.length, word, expectedText: word });
    }
  }
  return { issues, truncated: false };
}
