import { blankDocumentBytes } from '@docx-editor.dev/core/editor';
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import { headingFontSize } from './paragraph-styles';

export function createBlankDocument(): Uint8Array {
  const parts = unzipSync(blankDocumentBytes());
  const styles = strFromU8(parts['word/styles.xml']).replace(/<w:style\b[^>]*w:styleId="(Heading[1-9])"[^>]*>[\s\S]*?<\/w:style>/g, (style: string, styleId: string) => {
    const size = headingFontSize(styleId);
    if (!size) return style;
    return style.replace(/<w:rPr>([\s\S]*?)<\/w:rPr>/, (_properties: string, properties: string) => {
      const retained = properties.replace(/<w:sz(?:Cs)?\b[^>]*\/>/g, '');
      return `<w:rPr>${retained}<w:sz w:val="${size * 2}"/><w:szCs w:val="${size * 2}"/></w:rPr>`;
    });
  });
  parts['word/styles.xml'] = strToU8(styles);
  return zipSync(parts);
}
