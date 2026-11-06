import { describe, expect, it } from 'vitest';
import { matchAutoformat } from '../src/autoformat';

describe('Automatic Paragraph Formatting', () => {
  it.each(['*', '-', '+'])('Recognizes %s As A Bullet Prefix', (marker) => {
    expect(matchAutoformat(marker)).toEqual({ kind: 'bullet', prefix: marker });
  });

  it.each(['1.', '1)'])('Recognizes %s As A Numbered List Prefix', (marker) => {
    expect(matchAutoformat(marker)).toEqual({ kind: 'ordered', prefix: marker });
  });

  it.each([1, 2, 3, 4, 5, 6])('Recognizes A Level %s Heading Prefix', (level) => {
    const prefix = '#'.repeat(level);
    expect(matchAutoformat(prefix)).toEqual({ kind: 'heading', prefix, level });
  });

  it('Leaves Ordinary Text, Unsupported Markers, And Leading Whitespace Alone', () => {
    for (const prefix of ['', ' *', '**', 'Text *', '2.', '1.0', '01.', '#######', '>', '--', 'a.', 'I.']) {
      expect(matchAutoformat(prefix)).toBeNull();
    }
  });
});
