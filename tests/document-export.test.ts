import { describe, expect, it } from 'vitest';
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import { exportDocument } from '../src/document-export';

function fixture(): Uint8Array {
  return zipSync({
    '[Content_Types].xml': strToU8('<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>'),
    '_rels/.rels': strToU8('<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rDoc" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>'),
    'word/document.xml': strToU8('<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Résumé &amp; Notes</w:t></w:r></w:p><w:p><w:r><w:rPr><w:b/><w:i/></w:rPr><w:t>Bold 😀 {Text} \\ End</w:t></w:r><w:hyperlink r:id="rLink"><w:r><w:t>Safe Link</w:t></w:r></w:hyperlink><w:hyperlink r:id="rUnsafe"><w:r><w:t>Unsafe Link</w:t></w:r></w:hyperlink></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>List Item</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Cell B</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:drawing><wp:inline><wp:docPr id="1" name="Picture" descr="Test Picture"/><a:graphic><a:graphicData><a:blip r:embed="rImage"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p><w:sectPr/></w:body></w:document>'),
    'word/_rels/document.xml.rels': strToU8('<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/?a=1&amp;b=2" TargetMode="External"/><Relationship Id="rUnsafe" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="javascript:alert(1)" TargetMode="External"/><Relationship Id="rImage" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/picture.png"/></Relationships>'),
    'word/media/picture.png': new Uint8Array(Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAIAAAB7QOjdAAAAD0lEQVR4nGNUTX7NwMAAAAaHAXVGQY2IAAAAAElFTkSuQmCC', 'base64')),
    'word/numbering.xml': strToU8('<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>')
  });
}

describe('Portable Document Exports', () => {
  it('Keeps DOCX Bytes Unchanged', async () => {
    const source = fixture();
    const exported = await exportDocument(source, 'docx', 'Report');
    expect(exported.bytes).toEqual(source);
    expect(exported.extension).toBe('docx');
  });

  it('Produces UTF8 Text With Paragraphs And Table Cells', async () => {
    const result = await exportDocument(fixture(), 'txt', 'Report');
    const text = strFromU8(result.bytes);
    expect(text).toContain('Résumé & Notes');
    expect(text).toContain('Bold 😀 {Text} \\ End');
    expect(text).toContain('Cell A\tCell B');
    expect(text).not.toContain('<w:');
  });

  it('Creates A Self Contained HTML Archive With Safe Links And Images', async () => {
    const result = await exportDocument(fixture(), 'html', '<Report>');
    const files = unzipSync(result.bytes);
    const html = strFromU8(files['index.html']);
    expect(result.extension).toBe('zip');
    expect(html).toContain('<h1>Résumé &amp; Notes</h1>');
    expect(html).toContain('<strong><em>');
    expect(html).toContain('<table>');
    expect(html).toContain('<ul>');
    expect(html).toContain('https://example.com/?a=1&amp;b=2');
    expect(html).not.toContain('javascript:');
    expect(html).toContain('&lt;Report&gt;');
    expect(files['images/image-1.png']).toBeDefined();
    expect(html).toContain('src="images/image-1.png"');
  });

  it('Creates Markdown With Headings Lists Tables And Embedded Images', async () => {
    const markdown = strFromU8((await exportDocument(fixture(), 'md', 'Report')).bytes);
    expect(markdown).toContain('# Résumé & Notes');
    expect(markdown).toContain('***Bold 😀');
    expect(markdown).toContain('- List Item');
    expect(markdown).toContain('| Cell A | Cell B |');
    expect(markdown).toContain('data:image/png;base64,');
    expect(markdown).not.toContain('javascript:');
  });

  it('Writes An RTF Document With Unicode And Escaped Syntax', async () => {
    const rtf = strFromU8((await exportDocument(fixture(), 'rtf', 'Report')).bytes);
    expect(rtf).toMatch(/^\{\\rtf1\\ansi/);
    expect(rtf).toContain('\\u233?');
    expect(rtf).toContain('\\u-10179?\\u-8704?');
    expect(rtf).toContain('\\{Text\\}');
    expect(rtf).toContain('\\b ');
    expect(rtf).toContain('\\trowd');
    expect(rtf).toContain('\\pict\\pngblip');
    expect(rtf).not.toContain('javascript:');
  });

  it.each(['odt', 'epub'] as const)('Writes A Valid %s Container With Uncompressed First Mimetype', async (format) => {
    const result = await exportDocument(fixture(), format, 'Résumé & Notes');
    const files = unzipSync(result.bytes);
    const prefix = new DataView(result.bytes.buffer, result.bytes.byteOffset, result.bytes.byteLength);
    expect(prefix.getUint16(8, true)).toBe(0);
    expect(strFromU8(result.bytes.slice(30, 38))).toBe('mimetype');
    if (format === 'odt') {
      expect(strFromU8(files.mimetype)).toBe('application/vnd.oasis.opendocument.text');
      expect(strFromU8(files['content.xml'])).toContain('<text:h');
      expect(strFromU8(files['content.xml'])).toContain('<table:table');
      expect(strFromU8(files['content.xml'])).toContain('text:style-name="BoldItalic"');
      expect(strFromU8(files['META-INF/manifest.xml'])).toContain('Pictures/image-1.png');
      expect(files['Pictures/image-1.png']).toBeDefined();
    } else {
      expect(strFromU8(files.mimetype)).toBe('application/epub+zip');
      expect(strFromU8(files['META-INF/container.xml'])).toContain('EPUB/package.opf');
      expect(strFromU8(files['EPUB/package.opf'])).toContain('<spine>');
      expect(strFromU8(files['EPUB/package.opf'])).toContain('properties="nav"');
      expect(strFromU8(files['EPUB/content.xhtml'])).toContain('Résumé &amp; Notes');
      expect(files['EPUB/images/image-1.png']).toBeDefined();
    }
  });

  it('Rejects Invalid Input And Keeps PDF Generation With The Native Exporter', async () => {
    await expect(exportDocument(new Uint8Array([1]), 'html', 'Bad')).rejects.toThrow('Could Not Read');
    await expect(exportDocument(fixture(), 'pdf', 'Report')).rejects.toThrow('Native PDF');
  });

  it('Preserves Image Aspect Ratio In OpenDocument Downloads', async () => {
    const files = unzipSync((await exportDocument(fixture(), 'odt', 'Report')).bytes);
    const content = strFromU8(files['content.xml']);
    const dimensions = content.match(/svg:width="([\d.]+)cm" svg:height="([\d.]+)cm"/)!;
    expect(Number(dimensions[1]) / Number(dimensions[2])).toBeCloseTo(2, 1);
  });

  it('Keeps Numbered Lists Sequential In Rich Text And OpenDocument Files', async () => {
    const files = unzipSync(fixture());
    files['word/numbering.xml'] = strToU8(strFromU8(files['word/numbering.xml']).replace('w:val="bullet"', 'w:val="decimal"'));
    const paragraph = '<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>List Item</w:t></w:r></w:p>';
    files['word/document.xml'] = strToU8(strFromU8(files['word/document.xml']).replace(paragraph, paragraph + paragraph.replace('List Item', 'Second Item')));
    const bytes = zipSync(files);
    const rtf = strFromU8((await exportDocument(bytes, 'rtf', 'List')).bytes);
    expect(rtf).toContain('1.\\tab ');
    expect(rtf).toContain('2.\\tab ');
    const content = strFromU8(unzipSync((await exportDocument(bytes, 'odt', 'List')).bytes)['content.xml']);
    expect(content.match(/<text:list text:style-name="NumberedList"/g)).toHaveLength(1);
  });
});
