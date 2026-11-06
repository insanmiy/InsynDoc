import { describe, expect, it } from 'vitest';
import { strFromU8, unzipSync } from 'fflate';
import { createBlankDocument } from '../src/blank-document';

describe('New Document Heading Styles', () => {
  it('Authors App Heading Sizes In The DOCX Style Definitions', () => {
    const parts = unzipSync(createBlankDocument());
    const styles = strFromU8(parts['word/styles.xml']);
    for (const [index, size] of [26, 22, 18, 16, 14, 13, 12, 10, 9].entries()) {
      const definition = styles.match(new RegExp(`<w:style[^>]*w:styleId="Heading${index + 1}"[\\s\\S]*?<\\/w:style>`))?.[0];
      expect(definition).toContain(`<w:sz w:val="${size * 2}"/>`);
      expect(definition).toContain(`<w:szCs w:val="${size * 2}"/>`);
    }
    expect(strFromU8(parts['word/document.xml'])).not.toContain('<w:t>');
  });
});
