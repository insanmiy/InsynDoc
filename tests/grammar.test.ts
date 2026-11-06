import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { LocalLinter, Dialect } from 'harper.js';
import { slimBinaryInlined } from 'harper.js/slimBinaryInlined';
import { checkWriting, WRITING_LINT_CONFIG } from '../src/grammar-engine';

const linter = new LocalLinter({ binary: slimBinaryInlined, dialect: Dialect.American });
beforeAll(async () => { await linter.setLintConfig(WRITING_LINT_CONFIG); });
afterAll(async () => { await linter.dispose(); });
const check = (text: string) => checkWriting([{ blockId: 'body', text }], () => true, [], linter);

describe('Offline Sentence Context', () => {
  it('Does Not Police Offensive Language While Keeping Grammar Suggestions', async () => {
    const result = await check('That is a fucking good idea.');
    expect(result.issues.some((issue) => /offensive language/i.test(issue.message ?? ''))).toBe(false);
    expect((await check('This is a example.')).issues).toContainEqual(expect.objectContaining({ word: 'a', suggestions: ['an'] }));
  });
  it('Suggests Be In A Verb Slot Without Replacing Valid Me', async () => {
    const result = await check('I want to me there.');
    expect(result.issues).toContainEqual(expect.objectContaining({ word: 'me', start: 10, kind: 'grammar', suggestions: ['be'] }));
    expect((await check('It should me fine.')).issues).toContainEqual(expect.objectContaining({ word: 'me', suggestions: ['be'] }));
    for (const text of ['He gave it to me.', 'Please be there for me.', 'I want to be there.', 'Give it to me there.']) {
      expect((await check(text)).issues).toEqual([]);
    }
  });

  it('Uses The Grammar Engine For Real Word And Phrase Errors', async () => {
    expect((await check('Their is a cat.')).issues).toContainEqual(expect.objectContaining({ word: 'Their', suggestions: expect.arrayContaining(['There']) }));
    expect((await check('I could of helped.')).issues).toContainEqual(expect.objectContaining({ word: 'could of', suggestions: expect.arrayContaining(['could have']) }));
    expect((await check('This is a example.')).issues).toContainEqual(expect.objectContaining({ word: 'a', suggestions: ['an'] }));
  });

  it('Converts Grammar Offsets To UTF16 After Emoji', async () => {
    const text = '\u{1F642} This is a example.';
    const result = await check(text);
    const issue = result.issues.find((candidate) => candidate.word === 'a')!;
    expect(issue.start).toBe(text.indexOf('a example'));
    expect(text.slice(issue.start, issue.start + issue.length)).toBe(issue.expectedText);
  });

  it('Keeps Dictionary Words Separate From Contextual Ignore', async () => {
    const paragraphs = [{ blockId: 'body', text: 'I want to me there.' }];
    const result = await checkWriting(paragraphs, () => true, ['me'], linter);
    const issue = result.issues.find((candidate) => candidate.word === 'me')!;
    expect(issue).toBeDefined();
    expect((await checkWriting(paragraphs, () => true, [], linter, [issue.ignoreKey!])).issues).toEqual([]);
    expect((await checkWriting([{ ...paragraphs[0], text: 'It should me fine.' }], () => true, [], linter, [issue.ignoreKey!])).issues).not.toEqual([]);
  });

  it('Does Not Offer Grammar Edits Inside URLs Or Overlapping Misspellings', async () => {
    const text = 'https://example.com/their-is-a-example';
    expect((await check(text)).issues).toEqual([]);
    const result = await checkWriting([{ blockId: 'body', text: 'I want to me there.' }], (word) => word !== 'me', [], linter);
    expect(result.issues.filter((issue) => issue.word === 'me')).toHaveLength(1);
    expect(result.issues.find((issue) => issue.word === 'me')!.kind).toBeUndefined();
  });
});
