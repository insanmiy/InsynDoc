import { readFileSync } from 'node:fs';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import { suggestSpelling, type SpellingRequest, type SpellingResponse } from '../src/spelling-engine';

const worker = {
  onmessage: null as ((event: MessageEvent<SpellingRequest>) => Promise<void>) | null,
  postMessage: vi.fn<(response: SpellingResponse) => void>()
};

beforeAll(async () => {
  vi.stubGlobal('self', worker);
  vi.stubGlobal('fetch', async (url: URL) => new Response(readFileSync(new URL(
    url.pathname.endsWith('.aff') ? '../node_modules/dictionary-en/index.aff' : '../node_modules/dictionary-en/index.dic', import.meta.url
  ), 'utf8')));
  await import('../src/spelling.worker');
});
afterAll(() => vi.unstubAllGlobals());

async function suggest(word: string, personalWords: string[] = [], corrections: Array<{ source: string; replacement: string; count: number }> = []): Promise<string[]> {
  worker.postMessage.mockClear();
  await worker.onmessage!({ data: { type: 'suggest', id: 42, word, personalWords, corrections } } as MessageEvent<SpellingRequest>);
  const response = worker.postMessage.mock.calls[0][0];
  expect(response).toMatchObject({ type: 'suggested', id: 42 });
  if (response.type !== 'suggested') throw new Error('Expected Spelling Suggestions');
  return response.suggestions;
}

describe('Spelling Worker Suggestions', () => {
  it('Does Not Suppress Profanity From Spelling Suggestions', async () => {
    expect(await suggest('fuckk')).toContain('fuck');
    expect(await suggest('shitt')).toContain('shit');
  });
  it('Learns The Most Frequently Accepted Correction For Each Typo', async () => {
    const corrections = [
      { source: 'teh', replacement: 'tech', count: 4 },
      { source: 'teh', replacement: 'the', count: 1 }
    ];
    expect((await suggest('teh', [], corrections))[0]).toBe('tech');
    expect((await suggest('TEH', [], corrections))[0]).toBe('TECH');
    expect((await suggest('Teh', [], corrections))[0]).toBe('Tech');
    expect((await suggest('teh', [], [{ source: 'teh', replacement: 'Tech', count: 2 }]))[0]).toBe('tech');
    expect((await suggest('taht', [], corrections))[0]).toBe('that');
  });

  it('Includes A Learned Custom Word Even When The Dictionary Has No Match', async () => {
    expect((await suggest('InsynDco', [], [{ source: 'insyndco', replacement: 'InsynDoc', count: 1 }]))[0]).toBe('InsynDoc');
  });

  it.each([
    ['teh', 'the'], ['taht', 'that'], ['adn', 'and'], ['hte', 'the'],
    ['wiht', 'with'], ['yuo', 'you'], ['dont', "don't"], ['speling', 'spelling']
  ])('Ranks The Likely Correction For %s First', async (word, expected) => {
    expect((await suggest(word))[0]).toBe(expected);
  });

  it('Preserves Capitalization And Curly Apostrophes', async () => {
    expect((await suggest('Teh'))[0]).toBe('The');
    expect((await suggest('TEH'))[0]).toBe('THE');
    expect(await suggest('dosn\u2019t')).toContain('doesn\u2019t');
  });

  it('Suggests Nearby Personal Dictionary Words Without Unrelated Words', async () => {
    expect((await suggest('InsynDco', ['InsynDoc', 'Unrelated']))[0]).toBe('InsynDoc');
    expect(await suggest('insyndco', ['InsynDoc'])).toContain('InsynDoc');
    expect(await suggest('speling', ['Unrelated'])).not.toContain('Unrelated');
  });

  it('Preserves The Saved Casing Of Personal Names And Brands', async () => {
    expect(await suggest('Iphon', ['iPhone'])).toContain('iPhone');
    expect(await suggest('Ebayy', ['eBay'])).toContain('eBay');
    expect(await suggest('IPHON', ['iPhone'])).toContain('IPHONE');
  });

  it('Returns Unique Bounded Suggestions And Skips Valid Or Unsupported Input', async () => {
    const suggestions = await suggest('speling', ['spelling', 'spelling']);
    expect(suggestions.length).toBeLessThanOrEqual(5);
    expect(new Set(suggestions).size).toBe(suggestions.length);
    for (const word of ['', 'spelling', '1234', 'a'.repeat(65)]) {
      expect(await suggest(word)).toEqual([]);
    }
  });

  it('Suggests Long Personal Words Without Running Expensive Dictionary Generation', () => {
    const checker = { correct: () => false, suggest: vi.fn(() => []) };
    const personal = 'SupercalifragilisticexpialidociousDocument';
    expect(suggestSpelling('SupercalifragilisticexpialidociousDocumemt', checker, [personal])).toEqual([personal]);
    expect(checker.suggest).not.toHaveBeenCalled();
  });
});
