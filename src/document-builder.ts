import { strFromU8, strToU8, zipSync, type Zippable, type ZippableFile } from 'fflate';
import { blankDocumentParts } from './blank-document';

function partBytes(file: ZippableFile | undefined): Uint8Array {
  if (!file) return new Uint8Array();
  if (file instanceof Uint8Array) return file;
  if (Array.isArray(file)) return partBytes(file[0]);
  return new Uint8Array();
}

export interface DocxRun {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  link?: string;
}

export type DocxParagraphStyle = 'Normal' | 'Title' | 'Subtitle' | 'Quote' | 'Heading1' | 'Heading2' | 'Heading3' | 'Heading4' | 'Heading5' | 'Heading6';

export interface DocxParagraph {
  style?: DocxParagraphStyle;
  inlines: DocxRun[];
  align?: 'left' | 'center' | 'right' | 'justify';
  list?: { ordered: boolean; level: number };
  pageBreakBefore?: boolean;
}

export interface DocxTable {
  rows: DocxParagraph[][][];
}

export type DocxBlock = DocxParagraph | DocxTable;

const W_NS = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main';
const R_NS = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';

function isTable(block: DocxBlock): block is DocxTable {
  return 'rows' in block;
}

function escapeXml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function escapeAttribute(text: string): string {
  return escapeXml(text).replace(/"/g, '&quot;');
}

function safeHref(href: string | undefined): string | undefined {
  return href && /^(https?:|mailto:)/i.test(href) ? href : undefined;
}

function runXml(run: DocxRun): string {
  const properties: string[] = [];
  if (run.bold) properties.push('<w:b/>');
  if (run.italic) properties.push('<w:i/>');
  if (run.underline) properties.push('<w:u w:val="single"/>');
  if (run.strike) properties.push('<w:strike/>');
  const rpr = properties.length ? `<w:rPr>${properties.join('')}</w:rPr>` : '';
  const segments = run.text.split('\n');
  const content = segments.map((segment, index) => {
    const tabbed = escapeXml(segment).replace(/\t/g, '<w:tab/>');
    const text = tabbed ? `<w:t xml:space="preserve">${tabbed}</w:t>` : '';
    return `${index > 0 ? '<w:br/>' : ''}${text}`;
  }).join('');
  if (!content.trim() && !run.text) return '';
  return `<w:r>${rpr}${content}</w:r>`;
}

function paragraphXml(paragraph: DocxParagraph, linkIds: Map<string, string>): string {
  const properties: string[] = [];
  const style = paragraph.style && paragraph.style !== 'Normal' ? paragraph.style : paragraph.list ? 'ListParagraph' : undefined;
  if (style) properties.push(`<w:pStyle w:val="${style}"/>`);
  if (paragraph.align && paragraph.align !== 'left') {
    properties.push(`<w:jc w:val="${paragraph.align === 'justify' ? 'both' : paragraph.align}"/>`);
  }
  if (paragraph.list) {
    const numId = paragraph.list.ordered ? 2 : 1;
    const level = Math.max(0, Math.min(8, Math.floor(paragraph.list.level)));
    properties.push(`<w:numPr><w:ilvl w:val="${level}"/><w:numId w:val="${numId}"/></w:numPr>`);
  }
  if (paragraph.pageBreakBefore) properties.push('<w:pageBreakBefore/>');
  const ppr = properties.length ? `<w:pPr>${properties.join('')}</w:pPr>` : '';

  const runs: string[] = [];
  let pendingLink: string | undefined;
  let pendingRuns: DocxRun[] = [];
  const flush = (): void => {
    if (!pendingRuns.length) return;
    const inner = pendingRuns.map(runXml).join('');
    runs.push(pendingLink ? `<w:hyperlink r:id="${pendingLink}">${inner}</w:hyperlink>` : inner);
    pendingRuns = [];
  };
  for (const inline of paragraph.inlines) {
    const href = safeHref(inline.link);
    const linkId = href ? linkIds.get(href) : undefined;
    if ((linkId ?? undefined) !== pendingLink) { flush(); pendingLink = linkId; }
    pendingRuns.push(linkId ? { ...inline, link: undefined } : inline);
  }
  flush();
  return `<w:p>${ppr}${runs.join('')}</w:p>`;
}

const PAGE_TEXT_WIDTH_DXA = 9360;

function tableXml(table: DocxTable, linkIds: Map<string, string>): string {
  const rows = table.rows;
  const columns = Math.max(1, ...rows.map((row) => row.length));
  const columnWidth = Math.floor(PAGE_TEXT_WIDTH_DXA / columns);
  const grid = Array.from({ length: columns }, (_, index) => `<w:gridCol w:w="${index === columns - 1 ? PAGE_TEXT_WIDTH_DXA - columnWidth * (columns - 1) : columnWidth}"/>`).join('');
  const borders = ['top', 'left', 'bottom', 'right', 'insideH', 'insideV']
    .map((side) => `<w:${side} w:val="single" w:sz="4" w:space="0" w:color="auto"/>`).join('');
  const body = rows.map((row) => `<w:tr>${Array.from({ length: columns }, (_, index) => {
    const cell = row[index] ?? [];
    const content = cell.length ? cell.map((paragraph) => paragraphXml(paragraph, linkIds)).join('') : '<w:p/>';
    return `<w:tc><w:tcPr><w:tcW w:w="${Math.floor(5000 / columns)}" w:type="pct"/></w:tcPr>${content}</w:tc>`;
  }).join('')}</w:tr>`).join('');
  return `<w:tbl><w:tblPr><w:tblW w:w="5000" w:type="pct"/><w:tblBorders>${borders}</w:tblBorders><w:tblLayout w:type="fixed"/></w:tblPr><w:tblGrid>${grid}</w:tblGrid>${body}</w:tbl><w:p/>`;
}

function numberingLevels(format: 'bullet' | 'decimal'): string {
  return Array.from({ length: 9 }, (_, index) => {
    const marker = format === 'bullet' ? ['•', '◦', '▪'][index % 3] : `${Array.from({ length: index + 1 }, (_, level) => `%${level + 1}`).join('.')}.`;
    const left = 720 * (index + 1);
    return `<w:lvl w:ilvl="${index}"><w:start w:val="1"/><w:numFmt w:val="${format}"/><w:lvlText w:val="${marker}"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="${left}" w:hanging="360"/></w:pPr></w:lvl>`;
  }).join('');
}

function numberingXml(): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering xmlns:w="${W_NS}"><w:abstractNum w:abstractNumId="0">${numberingLevels('bullet')}</w:abstractNum><w:abstractNum w:abstractNumId="1">${numberingLevels('decimal')}</w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num></w:numbering>`;
}

export function buildDocx(blocks: readonly DocxBlock[]): Uint8Array {
  const parts = blankDocumentParts();
  const hrefs = new Set<string>();
  let usesLists = false;
  const scan = (paragraphs: readonly DocxParagraph[]): void => {
    for (const paragraph of paragraphs) {
      usesLists ||= Boolean(paragraph.list);
      for (const inline of paragraph.inlines) {
        const href = safeHref(inline.link);
        if (href) hrefs.add(href);
      }
    }
  };
  for (const block of blocks) {
    if (isTable(block)) for (const row of block.rows) for (const cell of row) scan(cell);
    else scan([block]);
  }
  const linkIds = new Map<string, string>();
  let nextId = usesLists ? 3 : 2;
  for (const href of hrefs) linkIds.set(href, `rId${nextId++}`);

  const blankDocument = strFromU8(partBytes(parts['word/document.xml']));
  const section = blankDocument.match(/<w:sectPr>[\s\S]*<\/w:sectPr>/)?.[0] ?? '';
  const body = blocks.map((block) => isTable(block) ? tableXml(block, linkIds) : paragraphXml(block, linkIds)).join('');
  parts['word/document.xml'] = strToU8(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="${W_NS}" xmlns:r="${R_NS}"><w:body>${body}${section}</w:body></w:document>`);

  const rels = strFromU8(partBytes(parts['word/_rels/document.xml.rels']));
  const relationshipXml: string[] = [];
  if (usesLists) {
    relationshipXml.push('<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>');
  }
  for (const [href, id] of linkIds) {
    relationshipXml.push(`<Relationship Id="${id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="${escapeAttribute(href)}" TargetMode="External"/>`);
  }
  parts['word/_rels/document.xml.rels'] = strToU8(rels.replace('</Relationships>', `${relationshipXml.join('')}</Relationships>`));

  if (usesLists) {
    parts['word/numbering.xml'] = strToU8(numberingXml());
    const types = strFromU8(partBytes(parts['[Content_Types].xml']));
    parts['[Content_Types].xml'] = strToU8(types.replace('</Types>', '<Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>'));
  }

  return zipSync(parts);
}
