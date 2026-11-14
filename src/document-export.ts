import { strToU8, zipSync, type Zippable } from 'fflate';
import { readExportModel, type ExportBlock, type ExportImage, type ExportInline, type ExportModel, type ExportParagraph } from './export-model';

export type ExportFormat = 'docx' | 'pdf' | 'odt' | 'txt' | 'rtf' | 'html' | 'epub' | 'md';

export interface ExportedDocument {
  bytes: Uint8Array;
  extension: string;
  mime: string;
}

function xml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[character]!);
}

function base64(bytes: Uint8Array): string {
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 8192) binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  return btoa(binary);
}

function imageData(image: ExportImage): string { return `data:${image.mime};base64,${base64(image.bytes)}`; }

function paragraphText(paragraph: ExportParagraph): string {
  return paragraph.inlines.map((inline) => inline.image ? `[${inline.image.alt}]` : inline.text).join('');
}

function plainText(blocks: readonly ExportBlock[]): string {
  return blocks.map((block) => block.kind === 'paragraph' ? paragraphText(block)
    : block.rows.map((row) => row.map((cell) => plainText(cell)).join('\t')).join('\n')).join('\n');
}

function inlineHtml(inline: ExportInline, imageSource: (image: ExportImage) => string): string {
  if (inline.image) return `<img src="${xml(imageSource(inline.image))}" alt="${xml(inline.image.alt)}" style="width:${inline.image.widthPt}pt;height:auto" />`;
  let text = xml(inline.text).replace(/\n/g, '<br />').replace(/\t/g, '&#9;');
  const style = inline.style;
  const declarations: string[] = [];
  if (style.color) declarations.push(`color:#${style.color}`);
  if (style.size) declarations.push(`font-size:${style.size}pt`);
  if (style.font) declarations.push(`font-family:${style.font}`);
  if (declarations.length) text = `<span style="${xml(declarations.join(';'))}">${text}</span>`;
  if (style.strike) text = `<s>${text}</s>`;
  if (style.underline) text = `<u>${text}</u>`;
  if (style.italic) text = `<em>${text}</em>`;
  if (style.bold) text = `<strong>${text}</strong>`;
  return inline.href ? `<a href="${xml(inline.href)}">${text}</a>` : text;
}

function htmlBlocks(blocks: readonly ExportBlock[], imageSource: (image: ExportImage) => string): string {
  let output = '';
  let list: { id: string; tag: string } | undefined;
  const closeList = (): void => { if (list) output += `</${list.tag}>`; list = undefined; };
  for (const block of blocks) {
    if (block.kind === 'table') {
      closeList();
      output += `<table><tbody>${block.rows.map((row) => `<tr>${row.map((cell) => `<td>${htmlBlocks(cell, imageSource)}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
      continue;
    }
    const text = block.inlines.map((inline) => inlineHtml(inline, imageSource)).join('') || '<br />';
    const styles = [block.alignment ? `text-align:${block.alignment}` : '', block.pageBreak ? 'break-before:page' : ''].filter(Boolean);
    if (block.list) {
      const tag = block.list.ordered ? 'ol' : 'ul';
      if (list?.id !== block.list.id || list.tag !== tag) { closeList(); output += `<${tag}>`; list = { id: block.list.id, tag }; }
      const indent = block.list.level ? ` style="margin-left:${block.list.level * 1.5}em"` : '';
      output += `<li${indent}>${text}</li>`;
      continue;
    }
    closeList();
    const tag = block.heading ? `h${block.heading}` : 'p';
    output += `<${tag}${styles.length ? ` style="${styles.join(';')}"` : ''}>${text}</${tag}>`;
  }
  closeList();
  return output;
}

const documentCss = '.export-document{font-family:Arial,sans-serif;font-size:11pt;line-height:1.5;color:#111;overflow-wrap:anywhere}.export-document p{margin:0 0 .75em;white-space:pre-wrap}.export-document h1,.export-document h2,.export-document h3,.export-document h4,.export-document h5,.export-document h6{line-height:1.2;break-after:avoid}.export-document table{border-collapse:collapse;width:100%;margin:1em 0}.export-document td{border:1px solid #999;padding:.4em;vertical-align:top}.export-document td p:last-child{margin-bottom:0}.export-document img{max-width:100%;height:auto}.export-document a{color:#164da8}.export-document li{white-space:pre-wrap}.export-document pre{white-space:pre-wrap}';

function htmlDocument(model: ExportModel, title: string, imageSource: (image: ExportImage) => string, xhtml = false): string {
  return `${xhtml ? '<?xml version="1.0" encoding="UTF-8"?>' : '<!DOCTYPE html>'}<html${xhtml ? ' xmlns="http://www.w3.org/1999/xhtml"' : ''} lang="en"><head><meta charset="UTF-8" /><title>${xml(title)}</title><style>${documentCss}</style></head><body><main class="export-document">${htmlBlocks(model.blocks, imageSource)}</main></body></html>`;
}

export async function exportHtmlDocument(bytes: Uint8Array, title: string): Promise<string> {
  return htmlDocument(readExportModel(bytes), title, imageData);
}

function markdownEscape(value: string): string { return value.replace(/([\\`*_{}\[\]<>|])/g, '\\$1'); }

function inlineMarkdown(inline: ExportInline): string {
  if (inline.image) return `![${markdownEscape(inline.image.alt)}](${imageData(inline.image)})`;
  let text = markdownEscape(inline.text);
  if (inline.style.bold) text = `**${text}**`;
  if (inline.style.italic) text = `*${text}*`;
  if (inline.style.strike) text = `~~${text}~~`;
  if (inline.href) text = `[${text}](${inline.href.replace(/ /g, '%20').replace(/\(/g, '%28').replace(/\)/g, '%29')})`;
  return text;
}

function markdown(blocks: readonly ExportBlock[]): string {
  const counters = new Map<string, number>();
  return blocks.map((block) => {
    if (block.kind === 'table') {
      const rows = block.rows.map((row) => row.map((cell) => markdown(cell).trim().replace(/\n/g, '<br />')));
      if (!rows.length) return '';
      const width = Math.max(...rows.map((row) => row.length));
      const row = (cells: readonly string[]) => `| ${Array.from({ length: width }, (_, index) => cells[index] ?? '').join(' | ')} |`;
      return [row(rows[0]), row(Array.from({ length: width }, () => '---')), ...rows.slice(1).map(row)].join('\n');
    }
    const text = block.inlines.map(inlineMarkdown).join('');
    if (block.heading) return `${'#'.repeat(block.heading)} ${text}`;
    if (block.list) {
      const count = (counters.get(block.list.id) ?? 0) + 1;
      counters.set(block.list.id, count);
      return `${'  '.repeat(block.list.level)}${block.list.ordered ? `${count}.` : '-'} ${text}`;
    }
    return text;
  }).join('\n\n') + '\n';
}

function rtfText(value: string): string {
  let output = '';
  for (let index = 0; index < value.length; index++) {
    const code = value.charCodeAt(index);
    const character = value[index];
    if (character === '\\' || character === '{' || character === '}') output += `\\${character}`;
    else if (character === '\n') output += '\\line ';
    else if (character === '\t') output += '\\tab ';
    else if (code < 32 || code > 126) output += `\\u${code > 32767 ? code - 65536 : code}?`;
    else output += character;
  }
  return output;
}

function rtfInline(inline: ExportInline): string {
  if (inline.image) {
    const kind = inline.image.mime === 'image/png' ? 'pngblip' : inline.image.mime === 'image/jpeg' ? 'jpegblip' : null;
    if (!kind) return rtfText(`[${inline.image.alt}]`);
    return `{\\pict\\${kind}\\picwgoal${Math.round(inline.image.widthPt * 20)}\\pichgoal${Math.round(inline.image.heightPt * 20)}\n${Array.from(inline.image.bytes, (value) => value.toString(16).padStart(2, '0')).join('')}\n}`;
  }
  const controls = `${inline.style.bold ? '\\b ' : ''}${inline.style.italic ? '\\i ' : ''}${inline.style.underline ? '\\ul ' : ''}${inline.style.strike ? '\\strike ' : ''}${inline.style.size ? `\\fs${Math.round(inline.style.size * 2)} ` : ''}`;
  const text = `{${controls}${rtfText(inline.text)}}`;
  return inline.href ? `{\\field{\\*\\fldinst HYPERLINK "${rtfText(inline.href).replace(/"/g, '%22')}"}{\\fldrslt ${text}}}` : text;
}

function rtfBlocks(blocks: readonly ExportBlock[], insideCell = false, counters = new Map<string, number>()): string {
  return blocks.map((block) => {
    if (block.kind === 'table') return block.rows.map((row) => `\\trowd\\trgaph108${row.map((_, index) => `\\cellx${Math.round(9000 * (index + 1) / row.length)}`).join('')}\n${row.map((cell) => `\\pard\\intbl ${rtfBlocks(cell, true, counters)}\\cell `).join('')}\\row\n`).join('');
    const alignment = ({ center: '\\qc ', right: '\\qr ', justify: '\\qj ' } as Record<string, string>)[block.alignment ?? ''] ?? '\\ql ';
    const heading = block.heading ? `\\b\\fs${Math.max(24, 44 - block.heading * 4)} ` : '\\fs22 ';
    const count = block.list ? (counters.get(block.list.id) ?? 0) + 1 : 0;
    if (block.list) counters.set(block.list.id, count);
    const marker = block.list ? `${' '.repeat(block.list.level * 2)}${block.list.ordered ? `${count}.` : '\\u8226?'}\\tab ` : '';
    return `{\\pard${insideCell ? '\\intbl' : ''}${alignment}${block.pageBreak ? '\\pagebb ' : ''}${heading}${marker}${block.inlines.map(rtfInline).join('')}\\par}\n`;
  }).join('');
}

function odtInline(inline: ExportInline): string {
  if (inline.image) return `<draw:frame draw:name="${xml(inline.image.filename)}" text:anchor-type="as-char" svg:width="${(inline.image.widthPt * 2.54 / 72).toFixed(4)}cm" svg:height="${(inline.image.heightPt * 2.54 / 72).toFixed(4)}cm"><draw:image xlink:href="Pictures/${xml(inline.image.filename)}" xlink:type="simple" xlink:show="embed" xlink:actuate="onLoad"/><svg:title>${xml(inline.image.alt)}</svg:title></draw:frame>`;
  let text = xml(inline.text).replace(/\t/g, '<text:tab/>').replace(/\n/g, '<text:line-break/>').replace(/ {2,}/g, (spaces) => `<text:s text:c="${spaces.length}"/>`);
  const style = inline.style.bold && inline.style.italic ? 'BoldItalic' : inline.style.bold ? 'Bold' : inline.style.italic ? 'Italic' : undefined;
  if (style) text = `<text:span text:style-name="${style}">${text}</text:span>`;
  if (inline.href) text = `<text:a xlink:type="simple" xlink:href="${xml(inline.href)}">${text}</text:a>`;
  return text;
}

function odtBlocks(blocks: readonly ExportBlock[], tableIndex: { value: number }): string {
  let listId: string | undefined;
  let output = '';
  const seenLists = new Set<string>();
  const closeList = (): void => { if (listId) output += '</text:list>'; listId = undefined; };
  for (const block of blocks) {
    if (block.kind === 'table') {
      closeList();
      tableIndex.value++;
      const columns = Math.max(0, ...block.rows.map((row) => row.length));
      output += `<table:table table:name="Table${tableIndex.value}"><table:table-column table:number-columns-repeated="${columns}"/>${block.rows.map((row) => `<table:table-row>${row.map((cell) => `<table:table-cell office:value-type="string">${odtBlocks(cell, tableIndex)}</table:table-cell>`).join('')}</table:table-row>`).join('')}</table:table>`;
      continue;
    }
    const text = block.inlines.map(odtInline).join('');
    const paragraph = `<text:p${block.alignment ? ` text:style-name="Align${block.alignment}"` : ''}>${text}</text:p>`;
    if (block.list) {
      if (listId !== block.list.id) {
        closeList();
        output += `<text:list text:style-name="${block.list.ordered ? 'NumberedList' : 'BulletList'}"${seenLists.has(block.list.id) ? ' text:continue-numbering="true"' : ''}>`;
        listId = block.list.id;
        seenLists.add(listId);
      }
      output += `<text:list-item>${paragraph}</text:list-item>`;
      continue;
    }
    closeList();
    output += block.heading ? `<text:h text:outline-level="${block.heading}" text:style-name="Heading${block.heading}">${text}</text:h>` : paragraph;
  }
  closeList();
  return output;
}

const odtNamespaces = 'xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"';

function odt(model: ExportModel, title: string): Uint8Array {
  const mime = 'application/vnd.oasis.opendocument.text';
  const textStyles = [['Bold', 'bold', 'normal'], ['Italic', 'normal', 'italic'], ['BoldItalic', 'bold', 'italic']].map(([name, weight, fontStyle]) => `<style:style style:name="${name}" style:family="text"><style:text-properties fo:font-weight="${weight}" fo:font-style="${fontStyle}"/></style:style>`).join('');
  const alignments = ['left', 'center', 'right', 'justify'].map((alignment) => `<style:style style:name="Align${alignment}" style:family="paragraph"><style:paragraph-properties fo:text-align="${alignment}"/></style:style>`).join('');
  const headings = Array.from({ length: 6 }, (_, index) => `<style:style style:name="Heading${index + 1}" style:family="paragraph"><style:paragraph-properties fo:margin-top="0.3cm" fo:margin-bottom="0.2cm"/><style:text-properties fo:font-size="${24 - index * 2}pt" fo:font-weight="bold"/></style:style>`).join('');
  const lists = '<text:list-style style:name="BulletList"><text:list-level-style-bullet text:level="1" text:bullet-char="•"/></text:list-style><text:list-style style:name="NumberedList"><text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/></text:list-style>';
  const files: Zippable = {
    mimetype: [strToU8(mime), { level: 0 }],
    'content.xml': strToU8(`<?xml version="1.0" encoding="UTF-8"?><office:document-content ${odtNamespaces} office:version="1.3"><office:automatic-styles>${textStyles}${alignments}${headings}${lists}</office:automatic-styles><office:body><office:text>${odtBlocks(model.blocks, { value: 0 })}</office:text></office:body></office:document-content>`),
    'styles.xml': strToU8(`<?xml version="1.0" encoding="UTF-8"?><office:document-styles ${odtNamespaces} office:version="1.3"><office:styles><style:default-style style:family="paragraph"><style:text-properties fo:font-size="11pt" fo:font-family="Arial"/></style:default-style></office:styles></office:document-styles>`),
    'meta.xml': strToU8(`<?xml version="1.0" encoding="UTF-8"?><office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/" office:version="1.3"><office:meta><dc:title>${xml(title)}</dc:title></office:meta></office:document-meta>`),
    'META-INF/manifest.xml': strToU8(`<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3"><manifest:file-entry manifest:full-path="/" manifest:media-type="${mime}"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>${model.images.map((image) => `<manifest:file-entry manifest:full-path="Pictures/${xml(image.filename)}" manifest:media-type="${image.mime}"/>`).join('')}</manifest:manifest>`)
  };
  for (const image of model.images) files[`Pictures/${image.filename}`] = image.bytes;
  return zipSync(files);
}

function epub(model: ExportModel, title: string): Uint8Array {
  const files: Zippable = {
    mimetype: [strToU8('application/epub+zip'), { level: 0 }],
    'META-INF/container.xml': strToU8('<?xml version="1.0" encoding="UTF-8"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="EPUB/package.opf" media-type="application/oebps-package+xml"/></rootfiles></container>'),
    'EPUB/content.xhtml': strToU8(htmlDocument(model, title, (image) => `images/${image.filename}`, true)),
    'EPUB/nav.xhtml': strToU8(`<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="en"><head><title>Contents</title></head><body><nav epub:type="toc" id="toc"><h1>Contents</h1><ol><li><a href="content.xhtml">${xml(title)}</a></li></ol></nav></body></html>`),
    'EPUB/package.opf': strToU8(`<?xml version="1.0" encoding="UTF-8"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="document-id"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="document-id">urn:uuid:${crypto.randomUUID()}</dc:identifier><dc:title>${xml(title)}</dc:title><dc:language>en</dc:language><meta property="dcterms:modified">${new Date().toISOString().replace(/\.\d{3}Z$/, 'Z')}</meta></metadata><manifest><item id="content" href="content.xhtml" media-type="application/xhtml+xml"/><item id="navigation" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>${model.images.map((image, index) => `<item id="image-${index + 1}" href="images/${xml(image.filename)}" media-type="${image.mime}"/>`).join('')}</manifest><spine><itemref idref="content"/></spine></package>`)
  };
  for (const image of model.images) files[`EPUB/images/${image.filename}`] = image.bytes;
  return zipSync(files);
}

export async function exportDocument(bytes: Uint8Array, format: ExportFormat, title: string): Promise<ExportedDocument> {
  if (format === 'pdf') throw new Error('Use the native PDF exporter to download this document.');
  if (format === 'docx') return { bytes: bytes.slice(), extension: 'docx', mime: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' };
  const model = readExportModel(bytes);
  if (format === 'txt') return { bytes: strToU8(plainText(model.blocks)), extension: 'txt', mime: 'text/plain;charset=utf-8' };
  if (format === 'md') return { bytes: strToU8(markdown(model.blocks)), extension: 'md', mime: 'text/markdown;charset=utf-8' };
  if (format === 'rtf') return { bytes: strToU8(`{\\rtf1\\ansi\\deff0\\uc1{\\fonttbl{\\f0 Arial;}}\n${rtfBlocks(model.blocks)}}`), extension: 'rtf', mime: 'application/rtf' };
  if (format === 'odt') return { bytes: odt(model, title), extension: 'odt', mime: 'application/vnd.oasis.opendocument.text' };
  if (format === 'epub') return { bytes: epub(model, title), extension: 'epub', mime: 'application/epub+zip' };
  const files: Zippable = { 'index.html': strToU8(htmlDocument(model, title, (image) => `images/${image.filename}`)) };
  for (const image of model.images) files[`images/${image.filename}`] = image.bytes;
  return { bytes: zipSync(files), extension: 'zip', mime: 'application/zip' };
}
