import { describe, expect, it } from 'vitest';
import { learnSpellingCorrection, parseSpellingCorrections } from '../src/spelling-learning';

describe('Local Spelling Learning', () => {
  it('Counts Accepted Pairs Without Changing Earlier History', () => {
    const first = learnSpellingCorrection([], 'Teh', 'tech');
    const second = learnSpellingCorrection(first, 'TEH', 'tech');
    expect(first).toEqual([{ source: 'teh', replacement: 'tech', count: 1 }]);
    expect(second).toEqual([{ source: 'teh', replacement: 'tech', count: 2 }]);
    expect(learnSpellingCorrection(second, 'teh', 'the')).toHaveLength(2);
  });

  it('Rejects Unchanged Words, Phrases, And Invalid Entries', () => {
    for (const replacement of ['', 'teh', 'two words', '<script>', 'a'.repeat(65)]) {
      expect(learnSpellingCorrection([], 'teh', replacement)).toEqual([]);
    }
    expect(parseSpellingCorrections([{}, null, { source: 'teh', replacement: 'the', count: Infinity }])).toEqual([]);
  });

  it('Bounds History And Counts And Merges Equivalent Keys', () => {
    expect(parseSpellingCorrections([
      { source: 'Teh', replacement: 'the', count: 9999 },
      { source: 'teh', replacement: 'THE', count: 2 }
    ])).toEqual([{ source: 'teh', replacement: 'the', count: 10000 }]);
    const history = Array.from({ length: 1000 }, (_, index) => ({ source: `typo${String.fromCharCode(0x4e00 + index)}`, replacement: 'word', count: 1 }));
    const updated = learnSpellingCorrection(history, 'teh', 'the');
    expect(updated).toHaveLength(1000);
    expect(updated[0]).toEqual({ source: 'teh', replacement: 'the', count: 1 });
  });
});
