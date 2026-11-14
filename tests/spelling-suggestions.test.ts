import { readFileSync } from 'node:fs';
import nspell from 'nspell';
import { describe, expect, it } from 'vitest';
import { checkParagraphs, createSpellingFallback, suggestSpelling } from '../src/spelling-engine';
import { learnSpellingCorrection } from '../src/spelling-learning';

const dictionary = readFileSync('node_modules/dictionary-en/index.dic', 'utf8');
const rules = readFileSync('node_modules/dictionary-en/index.aff', 'utf8').replace(/^NOSUGGEST[^\r\n]*(?:\r?\n|$)/gm, '');
const checker = nspell(rules, dictionary);
const enhancedChecker = { correct: (word: string) => checker.correct(word), suggest: (word: string) => checker.suggest(word), fallback: createSpellingFallback(dictionary) };

describe('Spelling Suggestions', () => {
  it('Finds Better Candidates When Dictionary Suggestions Are Weak', () => {
    const weakChecker = { correct: () => false, suggest: () => ['preface'], fallback: () => ['performance'] };
    expect(suggestSpelling('profromace', weakChecker)[0]).toBe('performance');
  });

  it('Favors Common Words And Adjacent Keyboard Mistakes', () => {
    const keyboardChecker = { correct: () => false, suggest: () => ['ward', 'word'] };
    expect(suggestSpelling('wprd', keyboardChecker)[0]).toBe('word');
    const commonChecker = { correct: () => false, suggest: () => ['ferm', 'form'] };
    expect(suggestSpelling('farmm', commonChecker)[0]).toBe('form');
  });

  it('Skips The Fallback For Close Suggestions And Oversized Words', () => {
    const fallback = () => { throw new Error('Fallback should not run.'); };
    expect(suggestSpelling('recieve', { correct: () => false, suggest: () => ['receive'], fallback })[0]).toBe('receive');
    expect(suggestSpelling('z'.repeat(40), { correct: () => false, suggest: () => [], fallback })).toEqual([]);
  });

  it.each([
    ['recieve', 'receive'], ['teh', 'the'], ['becuase', 'because'],
    ['definately', 'definitely'], ['seperate', 'separate'], ['occured', 'occurred'],
    ['adress', 'address'], ['enviroment', 'environment'], ['langauge', 'language'],
    ['documant', 'document'], ['writting', 'writing'], ['profromace', 'performance']
  ])('Includes %s To %s In The First Five Suggestions', (source, expected) => {
    expect(suggestSpelling(source, enhancedChecker)).toContain(expected);
  });

  it('Keeps Valid Words, Contractions, And Personal Names Unchanged', () => {
    for (const word of ['document', 'writing', "don't", 'can\u2019t', 'Windows']) {
      expect(suggestSpelling(word, enhancedChecker)).toEqual([]);
    }
    expect(suggestSpelling('InsynDoc', enhancedChecker, ['InsynDoc'])).toEqual([]);
  });

  it('Learns Ranking Without Adding An Accidental Correction To The Dictionary', () => {
    const corrections = learnSpellingCorrection([], 'profromace', 'zorbulate');
    expect(suggestSpelling('profromace', enhancedChecker, [], corrections)[0]).toBe('zorbulate');
    const paragraphs = [{ blockId: 'paragraph', text: 'zorbulate' }];
    expect(checkParagraphs(paragraphs, (word) => checker.correct(word), []).issues).toHaveLength(1);
    expect(checkParagraphs(paragraphs, (word) => checker.correct(word), ['zorbulate']).issues).toHaveLength(0);
  });

  it('Finds Performance For A Word With Several Typing Errors', () => {
    expect(suggestSpelling('profromace', enhancedChecker)).toContain('performance');
    expect(suggestSpelling('proformace', enhancedChecker)).toContain('performance');
  });

  it('Preserves Capitalization And Existing Dictionary Suggestions', () => {
    expect(suggestSpelling('PROFROMACE', enhancedChecker)).toContain('PERFORMANCE');
    expect(suggestSpelling('Profromace', enhancedChecker)).toContain('Performance');
    expect(suggestSpelling('recieve', enhancedChecker)[0]).toBe('receive');
  });

  it('Does Not Suggest Changes To Valid Or Personal Words', () => {
    expect(suggestSpelling('performance', enhancedChecker)).toEqual([]);
    expect(suggestSpelling('profromace', enhancedChecker, ['profromace'])).toEqual([]);
  });

  it('Keeps Learned Corrections First And Bounds Difficult Inputs', () => {
    expect(suggestSpelling('profromace', enhancedChecker, [], [{ source: 'profromace', replacement: 'performances', count: 2 }])[0]).toBe('performances');
    expect(enhancedChecker.fallback('z'.repeat(64))).toEqual([]);
    expect(enhancedChecker.fallback('aeiouaeiou')).toEqual([]);
    expect(suggestSpelling('123', enhancedChecker)).toEqual([]);
  });
});
