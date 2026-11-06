import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';

const WORD_DRAWING = 'http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing';
const PICTURE = 'http://schemas.openxmlformats.org/drawingml/2006/picture';
const DRAWING = 'http://schemas.openxmlformats.org/drawingml/2006/main';

export function synchronizeImageExtents(bytes: Uint8Array): Uint8Array {
  const parts = unzipSync(bytes);
  let changed = false;
  for (const [name, content] of Object.entries(parts)) {
    if (!/^word\/.*\.xml$/.test(name)) continue;
    const document = new DOMParser().parseFromString(strFromU8(content), 'application/xml');
    if (document.getElementsByTagName('parsererror').length) continue;
    let partChanged = false;
    for (const container of Array.from(document.getElementsByTagNameNS(WORD_DRAWING, '*'))) {
      if (container.localName !== 'inline' && container.localName !== 'anchor') continue;
      const extent = Array.from(container.children).find((child) => child.namespaceURI === WORD_DRAWING && child.localName === 'extent');
      const picture = container.getElementsByTagNameNS(PICTURE, 'pic')[0];
      const properties = picture && Array.from(picture.children).find((child) => child.namespaceURI === PICTURE && child.localName === 'spPr');
      const transform = properties && Array.from(properties.children).find((child) => child.namespaceURI === DRAWING && child.localName === 'xfrm');
      const imageExtent = transform && Array.from(transform.children).find((child) => child.namespaceURI === DRAWING && child.localName === 'ext');
      const width = extent?.getAttribute('cx');
      const height = extent?.getAttribute('cy');
      if (!imageExtent || !width || !height || !/^\d+$/.test(width) || !/^\d+$/.test(height)) continue;
      if (imageExtent.getAttribute('cx') === width && imageExtent.getAttribute('cy') === height) continue;
      imageExtent.setAttribute('cx', width);
      imageExtent.setAttribute('cy', height);
      partChanged = true;
    }
    if (partChanged) {
      parts[name] = strToU8(new XMLSerializer().serializeToString(document));
      changed = true;
    }
  }
  return changed ? zipSync(parts) : bytes;
}
