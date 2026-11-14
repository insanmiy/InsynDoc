import { openHeadlessDocument, resolveInternalTarget, validateRasterHeader } from '@docx-editor.dev/core/store';

interface XmlNode {
  readonly kind: string;
  readonly localName?: string;
  readonly namespaceUri?: string;
  readonly value?: string;
  readonly attributes?: readonly { readonly localName: string; readonly value: string }[];
  readonly children?: readonly XmlNode[];
}

export interface ExportTextStyle {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  color?: string;
  size?: number;
  font?: string;
}

export interface ExportImage {
  filename: string;
  bytes: Uint8Array;
  mime: 'image/png' | 'image/jpeg' | 'image/gif' | 'image/webp';
  alt: string;
  widthPt: number;
  heightPt: number;
}

export interface ExportInline {
  text: string;
  style: ExportTextStyle;
  href?: string;
  image?: ExportImage;
}

export interface ExportParagraph {
  kind: 'paragraph';
  inlines: ExportInline[];
  heading?: number;
  alignment?: string;
  list?: { id: string; ordered: boolean; level: number };
  pageBreak?: boolean;
}

export interface ExportTable {
  kind: 'table';
  rows: ExportBlock[][][];
}

export type ExportBlock = ExportParagraph | ExportTable;

export interface ExportModel {
  blocks: ExportBlock[];
  images: ExportImage[];
}

function children(node: XmlNode): readonly XmlNode[] { return node.children ?? []; }
function named(node: XmlNode, name: string): XmlNode | undefined { return children(node).find((child) => child.localName === name); }
function attribute(node: XmlNode | undefined, name: string): string | undefined { return node?.attributes?.find((entry) => entry.localName === name)?.value; }
function property(node: XmlNode | undefined, name: string): string | undefined { return node ? attribute(named(node, name), 'val') : undefined; }

function descendants(node: XmlNode, name: string): XmlNode[] {
  const result: XmlNode[] = [];
  const visit = (current: XmlNode): void => {
    if (current.localName === name) result.push(current);
    for (const child of children(current)) visit(child);
  };
  visit(node);
  return result;
}

function runStyle(node: XmlNode | undefined): ExportTextStyle {
  if (!node) return {};
  const style: ExportTextStyle = {};
  for (const [element, key] of [['b', 'bold'], ['i', 'italic'], ['u', 'underline'], ['strike', 'strike']] as const) {
    const value = named(node, element);
    if (value) style[key] = !['0', 'false', 'off', 'none'].includes(attribute(value, 'val') ?? '1');
  }
  const color = property(node, 'color');
  if (color && /^[a-f\d]{6}$/i.test(color)) style.color = color;
  const size = Number(property(node, 'sz')) / 2;
  if (Number.isFinite(size) && size > 0 && size <= 400) style.size = size;
  const font = attribute(named(node, 'rFonts'), 'ascii') ?? attribute(named(node, 'rFonts'), 'hAnsi');
  if (font && /^[\p{L}\p{N} _-]{1,80}$/u.test(font)) style.font = font;
  return style;
}

export function readExportModel(bytes: Uint8Array): ExportModel {
  const opened = openHeadlessDocument(bytes);
  if (!opened.ok) throw new Error('The document could not be read for export. Save a DOCX copy and try again.');
  const view = opened.view;
  const packageData = view.currentPackage();
  const images: ExportImage[] = [];
  const imageTargets = new Map<string, ExportImage>();
  const styles = new Map<string, XmlNode>();
  for (const style of view.stylesRoot() ? descendants(view.stylesRoot()!, 'style') : []) {
    const id = attribute(style, 'styleId');
    if (id) styles.set(id, style);
  }
  const inheritedStyle = (styleId: string | undefined, depth = 0): ExportTextStyle => {
    if (!styleId || depth > 12) return {};
    const style = styles.get(styleId);
    if (!style) return {};
    return { ...inheritedStyle(property(style, 'basedOn'), depth + 1), ...runStyle(named(style, 'rPr')) };
  };
  const orderedLists = new Map<string, boolean>();
  const numbering = view.numberingRoot();
  if (numbering) {
    for (const number of descendants(numbering, 'num')) {
      const id = attribute(number, 'numId');
      const abstractId = property(number, 'abstractNumId');
      const definition = descendants(numbering, 'abstractNum').find((entry) => attribute(entry, 'abstractNumId') === abstractId);
      if (id && definition) orderedLists.set(id, property(named(definition, 'lvl'), 'numFmt') !== 'bullet');
    }
  }
  const picture = (node: XmlNode): ExportImage | undefined => {
    const blip = descendants(node, 'blip')[0];
    const relationshipId = attribute(blip, 'embed');
    if (!relationshipId) return undefined;
    const target = view.relationshipTarget(relationshipId);
    if (!target || target.external) return undefined;
    const resolved = resolveInternalTarget(view.part().name, target.target);
    if (!resolved.ok) return undefined;
    const existing = imageTargets.get(resolved.partName);
    if (existing) return existing;
    const imageBytes = packageData.partBytes.get(resolved.partName);
    const extension = resolved.partName.split('.').pop()?.toLowerCase();
    const mime = extension ? ({ png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp' } as Record<string, ExportImage['mime']>)[extension] : undefined;
    if (!imageBytes || !mime) return undefined;
    const dimensions = validateRasterHeader(imageBytes, mime);
    if (!dimensions) return undefined;
    const extent = descendants(node, 'extent')[0];
    const authoredWidth = Number(attribute(extent, 'cx')) / 12700;
    const authoredHeight = Number(attribute(extent, 'cy')) / 12700;
    const hasAuthoredSize = Number.isFinite(authoredWidth) && Number.isFinite(authoredHeight) && authoredWidth > 0 && authoredHeight > 0;
    const sourceWidth = hasAuthoredSize ? authoredWidth : dimensions.pixelWidth * 0.75;
    const sourceHeight = hasAuthoredSize ? authoredHeight : dimensions.pixelHeight * 0.75;
    const widthPt = Math.min(450, sourceWidth);
    const heightPt = sourceHeight * widthPt / sourceWidth;
    const alt = attribute(descendants(node, 'docPr')[0], 'descr') ?? 'Image';
    const image = { filename: `image-${images.length + 1}.${extension === 'jpeg' ? 'jpg' : extension}`, bytes: imageBytes, mime, alt, widthPt, heightPt };
    images.push(image);
    imageTargets.set(resolved.partName, image);
    return image;
  };
  const inlines = (node: XmlNode, inherited: ExportTextStyle = {}, href?: string): ExportInline[] => {
    if (node.localName === 'r') {
      const styleId = property(named(node, 'rPr'), 'rStyle');
      const style = { ...inherited, ...inheritedStyle(styleId), ...runStyle(named(node, 'rPr')) };
      return children(node).filter((child) => child.localName !== 'rPr').flatMap((child) => inlines(child, style, href));
    }
    if (node.localName === 'hyperlink') {
      const id = attribute(node, 'id');
      const target = id ? view.relationshipTarget(id) : null;
      const anchor = attribute(node, 'anchor');
      const safeHref = target?.external && target.sinkSafe !== false && /^(https?:|mailto:)/i.test(target.target)
        ? target.target : anchor ? `#${encodeURIComponent(anchor)}` : undefined;
      return children(node).flatMap((child) => inlines(child, inherited, safeHref));
    }
    if (node.localName === 't') return [{ text: children(node).map((child) => child.value ?? '').join(''), style: inherited, href }];
    if (node.localName === 'tab') return [{ text: '\t', style: inherited, href }];
    if (node.localName === 'br' || node.localName === 'cr') return [{ text: '\n', style: inherited, href }];
    if (node.localName === 'drawing' || node.localName === 'pict') {
      const image = picture(node);
      return [{ text: image ? '' : '[Image]', style: inherited, image, href }];
    }
    if (['pPr', 'rPr', 'instrText', 'del', 'moveFrom'].includes(node.localName ?? '')) return [];
    return children(node).flatMap((child) => inlines(child, inherited, href));
  };
  const blocks = (node: XmlNode): ExportBlock[] => {
    if (node.localName === 'p') {
      const properties = named(node, 'pPr');
      const styleId = property(properties, 'pStyle');
      const outline = property(properties, 'outlineLvl');
      const heading = outline !== undefined ? Number(outline) + 1 : Number(styleId?.match(/^heading\s*([1-6])$/i)?.[1]);
      const numProperties = properties ? named(properties, 'numPr') : undefined;
      const listId = property(numProperties, 'numId');
      const alignment = property(properties, 'jc');
      return [{
        kind: 'paragraph', inlines: inlines(node, inheritedStyle(styleId)),
        heading: Number.isInteger(heading) && heading > 0 && heading <= 6 ? heading : undefined,
        alignment: alignment && ['left', 'center', 'right', 'both', 'justify'].includes(alignment) ? alignment === 'both' ? 'justify' : alignment : undefined,
        list: listId && listId !== '0' ? { id: listId, ordered: orderedLists.get(listId) ?? false, level: Math.min(8, Math.max(0, Number(property(numProperties, 'ilvl')) || 0)) } : undefined,
        pageBreak: Boolean(properties && named(properties, 'pageBreakBefore'))
      }];
    }
    if (node.localName === 'tbl') return [{ kind: 'table', rows: children(node).filter((child) => child.localName === 'tr').map((row) => children(row).filter((child) => child.localName === 'tc').map((cell) => children(cell).flatMap(blocks))) }];
    if (['sectPr', 'pPr', 'rPr', 'tcPr', 'tblPr', 'tblGrid', 'del', 'moveFrom'].includes(node.localName ?? '')) return [];
    return children(node).flatMap(blocks);
  };
  return { blocks: blocks(view.part().root), images };
}
