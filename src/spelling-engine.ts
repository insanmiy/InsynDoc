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
const commonDocumentWords = new Set('about after again also another back because before between both change check come day different document each edit end even example file find first form get give good great help important just know language last leave life like line list little long look make many most much name need new next number only open other over page part people performance place point read right same save say school see sentence set show small start still system take tell text thing think time use very want way well word work world write writing year'.split(' '));
const keyboardPositions = new Map<string, { row: number; column: number }>();
for (const [row, letters] of ['qwertyuiop', 'asdfghjkl', 'zxcvbnm'].entries()) {
  for (const [column, letter] of Array.from(letters).entries()) {
    keyboardPositions.set(letter, { row, column: column + row * 0.25 });
  }
}
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
      const sourcePosition = keyboardPositions.get(source[sourceIndex - 1]);
      const targetPosition = keyboardPositions.get(target[targetIndex - 1]);
      const adjacent = sourcePosition && targetPosition
        && Math.abs(sourcePosition.row - targetPosition.row) <= 1
        && Math.abs(sourcePosition.column - targetPosition.column) <= 1;
      const substitutionCost = source[sourceIndex - 1] === target[targetIndex - 1] ? 0 : adjacent ? 0.65 : 1;
      current[targetIndex] = Math.min(
        previous[targetIndex] + 1,
        current[targetIndex - 1] + insertionCost,
        previous[targetIndex - 1] + substitutionCost
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

export function createSpellingFallback(dictionary: string): (word: string) => string[] {
  const wordsByLength = new Map<number, { word: string; consonants: string }[]>();
  for (const entry of dictionary.split(/\r?\n/)) {
    const word = entry.split('/')[0];
    if (!/^[a-z]{6,26}$/.test(word)) continue;
    const words = wordsByLength.get(word.length) ?? [];
    words.push({ word, consonants: word.replace(/[aeiouy]/g, '') });
    wordsByLength.set(word.length, words);
  }
  const cache = new Map<string, string[]>();
  return (word) => {
    const key = spellingKey(word);
    if (!/^[a-z]{6,24}$/.test(key)) return [];
    const cached = cache.get(key);
    if (cached) return [...cached];
    const consonants = key.replace(/[aeiouy]/g, '');
    if (consonants.length < 3) return [];
    const candidates: { word: string; distance: number }[] = [];
    const maximumDistance = Math.min(4, Math.floor(key.length / 2.5));
    for (let length = key.length - 2; length <= key.length + 2; length++) {
      for (const candidate of wordsByLength.get(length) ?? []) {
        if (Math.abs(candidate.consonants.length - consonants.length) > 1) continue;
        if (suggestionDistance(consonants, candidate.consonants) > 1) continue;
        const distance = suggestionDistance(key, candidate.word);
        if (distance <= maximumDistance) candidates.push({ word: candidate.word, distance });
      }
    }
    const suggestions = candidates.sort((left, right) => left.distance - right.distance).slice(0, 5).map((candidate) => candidate.word);
    if (cache.size >= 128) cache.delete(cache.keys().next().value!);
    cache.set(key, suggestions);
    return [...suggestions];
  };
}

export function suggestSpelling(word: string, checker: { correct: (word: string) => boolean; suggest: (word: string) => string[]; fallback?: (word: string) => string[] }, personalWords: readonly string[] = [], corrections: readonly SpellingCorrection[] = []): string[] {
  if (!isSpellingWord(word)) return [];
  const normalized = word.replace(/\u2019/g, "'");
  const key = spellingKey(word);
  if (checker.correct(normalized) || personalWords.some((personal) => spellingKey(personal) === key)) return [];
  // Keep Expensive Dictionary Generation Bounded, Even For Long Personal Words.
  const candidates = normalized.length <= 32 ? checker.suggest(normalized).slice(0, 64) : [];
  if (normalized.length <= 24 && !candidates.some((candidate) => suggestionDistance(key, spellingKey(candidate)) <= 1.25)) {
    candidates.push(...(checker.fallback?.(normalized) ?? []).slice(0, 5));
  }
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
    score: suggestionDistance(key, spellingKey(candidate))
      - (commonWords.has(spellingKey(candidate)) || commonDocumentWords.has(spellingKey(candidate)) ? 0.35 : 0)
  })).sort((left, right) => right.learned - left.learned || left.score - right.score || left.index - right.index);
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
