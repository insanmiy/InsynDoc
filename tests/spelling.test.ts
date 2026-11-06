import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import nspell from 'nspell';
import { checkParagraphs, spellingKey } from '../src/spelling-engine';

const checker = nspell(
  readFileSync(new URL('../node_modules/dictionary-en/index.aff', import.meta.url), 'utf8'),
  readFileSync(new URL('../node_modules/dictionary-en/index.dic', import.meta.url), 'utf8')
);

describe('Offline English Spelling', () => {
  it('Finds Misspellings And Offers Real Dictionary Suggestions', () => {
    const result = checkParagraphs([{ blockId: 'body', text: 'This is a speling test.' }], (word) => checker.correct(word), []);
    expect(result.issues).toEqual([{ blockId: 'body', start: 10, length: 7, word: 'speling', expectedText: 'speling' }]);
    expect(checker.suggest('speling')).toContain('spelling');
    expect(checkParagraphs([{ blockId: 'body', text: 'This is a spelling test.' }], (word) => checker.correct(word), []).issues).toEqual([]);
  });

  it('Keeps Model Offsets Across Unicode And Normalizes Apostrophes', () => {
    const result = checkParagraphs([{ blockId: 'body', text: '\u{1F642} speling doesn\u2019t' }], (word) => checker.correct(word), []);
    expect(result.issues).toHaveLength(1);
    expect(result.issues[0]).toMatchObject({ word: 'speling', start: 3, length: 7 });
    expect(spellingKey('Writer\u2019s')).toBe("writer's");
  });

  it('Ignores Web Addresses, Email Addresses, Numbers, And Personal Words', () => {
    const result = checkParagraphs([{ blockId: 'body', text: 'https://speling.example/path person@speling.example 1234 InsynDoc speling' }], (word) => checker.correct(word), ['insyndoc']);
    expect(result.issues.map((issue) => issue.word)).toEqual(['speling']);
  });

  it('Caches Repeated Words And Bounds Painted Issues', () => {
    let calls = 0;
    const result = checkParagraphs([{ blockId: 'body', text: 'speling '.repeat(1100) }], () => { calls++; return false; }, []);
    expect(calls).toBe(1);
    expect(result.issues).toHaveLength(1000);
    expect(result.truncated).toBe(true);
  });
});
