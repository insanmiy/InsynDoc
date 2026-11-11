import { strFromU8, unzipSync } from 'fflate';
import { buildDocx, type DocxBlock, type DocxParagraph, type DocxParagraphStyle, type DocxRun, type DocxTable } from './document-builder';

export type ImportFormat = 'md' | 'markdown' | 'html' | 'htm' | 'txt' | 'odt';

const paragraph = (inlines: DocxRun[], extra?: Partial<DocxParagraph>): DocxParagraph => ({ inlines, ...extra });
const text = (value: string): DocxRun[] => value ? [{ text: value }] : [];

// Inline Markdown Parsing: Bold, Italic, Strikethrough, Code, Links, Images.
const INLINE_PATTERN = /(\*\*[^*]+\*\*|__[^_]+__|\*[^*\n]+\*|_[^_\n]+_|~~[^~\n]+~~|`[^`\n]+`|!?\[[^\]\n]*\]\([^)\n]*\))/g;

function parseInlineMarkdown(source: string): DocxRun[] {
  const runs: DocxRun[] = [];
  let index = 0;
  for (const match of source.matchAll(INLINE_PATTERN)) {
    const start = match.index ?? 0;
    if (start > index) runs.push(...text(source.slice(index, start)));
    const token = match[0];
    index = start + token.length;
    if (token.startsWith('**') || token.startsWith('__')) {
      runs.push(...parseInlineMarkdown(token.slice(2, -2)).map((run) => ({ ...run, bold: true })));
    } else if (token.startsWith('*') || token.startsWith('_')) {
      runs.push(...parseInlineMarkdown(token.slice(1, -1)).map((run) => ({ ...run, italic: true })));
    } else if (token.startsWith('~~')) {
      runs.push(...parseInlineMarkdown(token.slice(2, -2)).map((run) => ({ ...run, strike: true })));
    } else if (token.startsWith('`')) {
      runs.push({ text: token.slice(1, -1) });
    } else if (token.startsWith('![')) {
      const alt = token.slice(2, token.indexOf(']'));
      runs.push({ text: alt ? `[Image: ${alt}]` : '[Image]' });
    } else {
      const close = token.indexOf('](');
      const label = token.slice(1, close);
      const href = token.slice(close + 2, -1);
      const inner = parseInlineMarkdown(label);
      if (/^(https?:|mailto:)/i.test(href)) {
        for (const run of inner) runs.push({ ...run, link: href });
      } else {
        runs.push(...inner);
      }
    }
  }
  if (index < source.length) runs.push(...text(source.slice(index)));
  return runs;
}

// Markdown Block Parsing.
const HEADING_PATTERN = /^(#{1,6})\s+(.*)$/;
const UNORDERED_PATTERN = /^(\s*)[-*+]\s+(.*)$/;
const ORDERED_PATTERN = /^(\s*)\d{1,9}[.)]\s+(.*)$/;
const QUOTE_PATTERN = /^>\s?(.*)$/;
const TABLE_SEPARATOR = /^\s*\|?[\s:|-]+\|[\s:|-]*$/;

function isTableRow(line: string): boolean {
  return line.includes('|') && line.trim().length > 0;
}

function splitTableRow(line: string): string[] {
  let value = line.trim();
  if (value.startsWith('|')) value = value.slice(1);
  if (value.endsWith('|')) value = value.slice(0, -1);
  return value.split('|').map((cell) => cell.trim());
}

function parseMarkdown(source: string): DocxBlock[] {
  const blocks: DocxBlock[] = [];
  const lines = source.replace(/\r\n?/g, '\n').split('\n');
  let inCode = false;
  let codeLines: string[] = [];
  const flushCode = (): void => {
    if (!codeLines.length) return;
    blocks.push(paragraph(text(codeLines.join('\n'))));
    codeLines = [];
  };
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    if (/^```/.test(line.trim())) {
      if (inCode) { flushCode(); inCode = false; }
      else { inCode = true; codeLines = []; }
      continue;
    }
    if (inCode) { codeLines.push(line); continue; }
    if (!line.trim()) { flushCode(); continue; }
    const heading = line.match(HEADING_PATTERN);
    if (heading) {
      flushCode();
      const level = Math.min(6, heading[1].length) as 1 | 2 | 3 | 4 | 5 | 6;
      blocks.push(paragraph(parseInlineMarkdown(heading[2]), { style: `Heading${level}` as DocxParagraphStyle }));
      continue;
    }
    const quote = line.match(QUOTE_PATTERN);
    if (quote) {
      flushCode();
      blocks.push(paragraph(parseInlineMarkdown(quote[1]), { style: 'Quote' }));
      continue;
    }
    const unordered = line.match(UNORDERED_PATTERN);
    const ordered = line.match(ORDERED_PATTERN);
    const list = unordered ?? ordered;
    if (list) {
      flushCode();
      const indent = list[1].replace(/\t/g, '  ').length;
      const level = Math.min(8, Math.floor(indent / 2));
      blocks.push(paragraph(parseInlineMarkdown(list[2]), { list: { ordered: Boolean(ordered), level } }));
      continue;
    }
    if (isTableRow(line) && index + 1 < lines.length && TABLE_SEPARATOR.test(lines[index + 1]) && lines[index + 1].includes('-')) {
      flushCode();
      const rows: DocxParagraph[][][] = [splitTableRow(line).map((cell) => [paragraph(parseInlineMarkdown(cell))])];
      index += 2;
      while (index < lines.length && isTableRow(lines[index]) && lines[index].trim()) {
        rows.push(splitTableRow(lines[index]).map((cell) => [paragraph(parseInlineMarkdown(cell))]));
        index++;
      }
      index--;
      blocks.push({ rows } satisfies DocxTable);
      continue;
    }
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(line.trim())) { flushCode(); continue; }
    flushCode();
    blocks.push(paragraph(parseInlineMarkdown(line)));
  }
  flushCode();
  return blocks;
}

function parsePlainText(source: string): DocxBlock[] {
  const blocks: DocxBlock[] = [];
  for (const chunk of source.replace(/\r\n?/g, '\n').split(/\n{2,}/)) {
    const lines = chunk.split('\n');
    if (lines.length === 1 && !lines[0].trim()) continue;
    const inlines: DocxRun[] = [];
    lines.forEach((line, index) => {
      if (index > 0) inlines.push({ text: '\n' });
      inlines.push(...text(line));
    });
    blocks.push(paragraph(inlines));
  }
  return blocks.length ? blocks : [paragraph([])];
}

// HTML Import.
function localName(node: Node): string {
  return node.nodeType === 1 ? (node as Element).localName.toLowerCase() : '';
}

interface InlineStyle { bold?: boolean; italic?: boolean; underline?: boolean; strike?: boolean; link?: string }

function htmlInlines(node: Node, style: InlineStyle = {}): DocxRun[] {
  const runs: DocxRun[] = [];
  const visit = (current: Node, inherited: InlineStyle): void => {
    if (current.nodeType === 3) {
      const value = current.nodeValue ?? '';
      if (value) runs.push({ text: value, ...inherited });
      return;
    }
    if (current.nodeType !== 1) return;
    const name = localName(current);
    if (name === 'br') { runs.push({ text: '\n', ...inherited }); return; }
    if (name === 'img') {
      const alt = (current as Element).getAttribute('alt');
      runs.push({ text: alt ? `[Image: ${alt}]` : '[Image]', ...inherited });
      return;
    }
    if (name === 'script' || name === 'style') return;
    const next: InlineStyle = { ...inherited };
    if (name === 'strong' || name === 'b') next.bold = true;
    if (name === 'em' || name === 'i') next.italic = true;
    if (name === 'u' || name === 'ins') next.underline = true;
    if (name === 'del' || name === 's' || name === 'strike') next.strike = true;
    if (name === 'a') {
      const href = (current as Element).getAttribute('href') ?? undefined;
      if (href && /^(https?:|mailto:)/i.test(href)) next.link = href;
    }
    if (name === 'code' || name === 'kbd' || name === 'samp') {
      for (const child of Array.from(current.childNodes)) visit(child, next);
      return;
    }
    for (const child of Array.from(current.childNodes)) visit(child, next);
  };
  visit(node, style);
  return runs;
}

const BLOCK_TAGS = new Set(['p', 'div', 'section', 'article', 'header', 'footer', 'main', 'aside', 'blockquote', 'pre', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'ul', 'ol', 'table', 'hr', 'figure', 'figcaption', 'dl', 'dt', 'dd']);

function htmlBlocks(node: Node, quote = false, listLevel = 0): DocxBlock[] {
  const blocks: DocxBlock[] = [];
  for (const child of Array.from(node.childNodes)) {
    if (child.nodeType === 3) {
      const value = (child.nodeValue ?? '').trim();
      if (value) blocks.push(paragraph(text(value), quote ? { style: 'Quote' } : undefined));
      continue;
    }
    if (child.nodeType !== 1) continue;
    const name = localName(child);
    if (/^h[1-6]$/.test(name)) {
      blocks.push(paragraph(htmlInlines(child), { style: `Heading${name[1]}` as DocxParagraphStyle }));
      continue;
    }
    if (name === 'ul' || name === 'ol') {
      const ordered = name === 'ol';
      const list = child as Element;
      for (const item of Array.from(list.children)) {
        if (localName(item) !== 'li') continue;
        const inlineNodes = Array.from(item.childNodes).filter((entry) => {
          if (entry.nodeType !== 1) return true;
          return !['ul', 'ol'].includes(localName(entry as Node));
        });
        const wrapper = item.ownerDocument.createElement('span');
        for (const entry of inlineNodes) wrapper.appendChild((entry as Node).cloneNode(true));
        const inlines = htmlInlines(wrapper);
        if (inlines.length) blocks.push(paragraph(inlines, { list: { ordered, level: listLevel } }));
        for (const entry of Array.from(item.children)) {
          if (['ul', 'ol'].includes(localName(entry))) blocks.push(...htmlBlocks(entry, quote, listLevel + 1));
        }
      }
      continue;
    }
    if (name === 'table') {
      const rows: DocxParagraph[][][] = [];
      for (const row of Array.from((child as Element).children)) {
        if (localName(row) !== 'tr') continue;
        const cells: DocxParagraph[][] = [];
        for (const cell of Array.from(row.children)) {
          if (!['td', 'th'].includes(localName(cell))) continue;
          cells.push(htmlBlocks(cell, false, 0).filter((block): block is DocxParagraph => !('rows' in block)));
        }
        if (cells.length) rows.push(cells);
      }
      if (rows.length) blocks.push({ rows } satisfies DocxTable);
      continue;
    }
    if (name === 'hr') continue;
    if (BLOCK_TAGS.has(name)) {
      const inner = htmlBlocks(child, quote || name === 'blockquote', listLevel);
      if (inner.length) blocks.push(...inner);
      else if (name === 'pre') blocks.push(paragraph([]));
      continue;
    }
    // Inline Or Unknown Element: Collect Runs Into A Paragraph.
    const inlines = htmlInlines(child);
    if (inlines.length) blocks.push(paragraph(inlines, quote ? { style: 'Quote' } : undefined));
  }
  return blocks;
}

function parseHtml(source: string): DocxBlock[] {
  const document = new DOMParser().parseFromString(source, 'text/html');
  const body = document.body;
  if (!body) return [];
  const blocks = htmlBlocks(body);
  return blocks.length ? blocks : [paragraph([])];
}

// ODT Import.
const OFFICE_NS = 'urn:oasis:names:tc:opendocument:xmlns:office:1.0';

type OdtTextStyles = Map<string, { bold: boolean; italic: boolean }>;

function odtTextStyles(document: Document): OdtTextStyles {
  const styles: OdtTextStyles = new Map();
  for (const style of Array.from(document.getElementsByTagNameNS('*', 'style'))) {
    if (style.getAttribute('style:family') !== 'text') continue;
    const name = style.getAttribute('style:name');
    if (!name) continue;
    const properties = style.getElementsByTagNameNS('*', 'text-properties')[0];
    const weight = properties?.getAttribute('fo:font-weight') ?? '';
    const fontStyle = properties?.getAttribute('fo:font-style') ?? '';
    styles.set(name.toLowerCase(), {
      bold: weight === 'bold' || Number(weight) >= 600,
      italic: fontStyle === 'italic' || fontStyle === 'oblique'
    });
  }
  return styles;
}

function odtStyleName(element: Element): string {
  return (element.getAttribute('text:style-name') ?? element.getAttribute('style-name') ?? '').toLowerCase();
}

function odtInlines(node: Node, styles: OdtTextStyles): DocxRun[] {
  const runs: DocxRun[] = [];
  const visit = (current: Node, style: InlineStyle): void => {
    if (current.nodeType === 3) {
      const value = current.nodeValue ?? '';
      if (value.trim() || value.includes(' ')) runs.push({ text: value, ...style });
      return;
    }
    if (current.nodeType !== 1) return;
    const name = localName(current);
    if (name === 's') { runs.push({ text: ' ', ...style }); return; }
    if (name === 'tab') { runs.push({ text: '\t', ...style }); return; }
    if (name === 'line-break') { runs.push({ text: '\n', ...style }); return; }
    const styleName = odtStyleName(current as Element);
    const resolved = styles.get(styleName);
    const next: InlineStyle = { ...style };
    if (resolved?.bold || /bold/.test(styleName)) next.bold = true;
    if (resolved?.italic || /italic/.test(styleName)) next.italic = true;
    if (name === 'a') {
      const href = (current as Element).getAttribute('xlink:href') ?? (current as Element).getAttribute('href');
      if (href && /^(https?:|mailto:)/i.test(href)) next.link = href;
    }
    for (const child of Array.from(current.childNodes)) visit(child, next);
  };
  visit(node, {});
  return runs;
}

function odtParagraph(element: Element, styles: OdtTextStyles): DocxParagraph {
  const name = localName(element);
  const level = name === 'h' ? Math.min(6, Math.max(1, Number(element.getAttribute('text:outline-level') ?? '0') + 1)) : 0;
  return paragraph(odtInlines(element, styles), level ? { style: `Heading${level}` as DocxParagraphStyle } : undefined);
}

function odtBlocks(node: Node, styles: OdtTextStyles, listLevel = 0): DocxBlock[] {
  const blocks: DocxBlock[] = [];
  for (const child of Array.from(node.childNodes)) {
    if (child.nodeType !== 1) continue;
    const name = localName(child);
    if (name === 'h' || name === 'p') {
      const item = odtParagraph(child as Element, styles);
      if (item.inlines.length || name === 'p') blocks.push(item);
      continue;
    }
    if (name === 'list') {
      for (const item of Array.from((child as Element).children)) {
        if (localName(item) !== 'list-item') continue;
        const element = item as Element;
        let emitted = false;
        for (const entry of Array.from(element.childNodes)) {
          if (entry.nodeType !== 1) continue;
          const entryName = localName(entry);
          if (entryName === 'p' || entryName === 'h') {
            const itemParagraph = odtParagraph(entry as Element, styles);
            blocks.push({ ...itemParagraph, list: { ordered: false, level: listLevel } });
            emitted = true;
          } else if (entryName === 'list') {
            blocks.push(...odtBlocks(entry, styles, listLevel + 1));
          }
        }
        if (!emitted) blocks.push(paragraph([], { list: { ordered: false, level: listLevel } }));
      }
      continue;
    }
    if (name === 'table') {
      const rows: DocxParagraph[][][] = [];
      for (const row of Array.from((child as Element).children)) {
        if (localName(row) !== 'table-row') continue;
        const cells: DocxParagraph[][] = [];
        for (const cell of Array.from((row as Element).children)) {
          if (!['table-cell', 'covered-table-cell'].includes(localName(cell))) continue;
          cells.push(odtBlocks(cell, styles).filter((block): block is DocxParagraph => !('rows' in block)));
        }
        if (cells.length) rows.push(cells);
      }
      if (rows.length) blocks.push({ rows } satisfies DocxTable);
      continue;
    }
    if (name === 'section') {
      blocks.push(...odtBlocks(child, styles, listLevel));
    }
  }
  return blocks;
}

function parseOdt(bytes: Uint8Array): DocxBlock[] {
  const parts = unzipSync(bytes);
  const content = parts['content.xml'];
  if (!content) throw new Error('The ODT File Has No Readable Content.');
  const document = new DOMParser().parseFromString(strFromU8(content), 'text/xml');
  if (document.getElementsByTagName('parsererror').length) throw new Error('The ODT File Could Not Be Read.');
  const text = document.getElementsByTagNameNS(OFFICE_NS, 'text')[0];
  const blocks = text ? odtBlocks(text, odtTextStyles(document)) : [];
  return blocks.length ? blocks : [paragraph([])];
}

export function importBlocks(_name: string, bytes: Uint8Array, format: ImportFormat | string): DocxBlock[] {
  switch (format) {
    case 'md':
    case 'markdown':
      return parseMarkdown(new TextDecoder('utf-8').decode(bytes));
    case 'html':
    case 'htm':
      return parseHtml(new TextDecoder('utf-8').decode(bytes));
    case 'txt':
      return parsePlainText(new TextDecoder('utf-8').decode(bytes));
    case 'odt':
      return parseOdt(bytes);
    default:
      throw new Error('This File Type Cannot Be Imported.');
  }
}

export function importFileName(name: string): string {
  const dot = name.lastIndexOf('.');
  const stem = dot > 0 ? name.slice(0, dot) : name;
  const clean = stem.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '').trim();
  return `${clean || 'Imported'}.docx`;
}

export async function importDocument(name: string, bytes: Uint8Array, format: ImportFormat | string): Promise<{ bytes: Uint8Array; name: string }> {
  const blocks = importBlocks(name, bytes, format);
  return { bytes: buildDocx(blocks.length ? blocks : [{ inlines: [] }]), name: importFileName(name) };
}
