import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';

const CORE_PROPERTIES = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title></dc:title><dc:subject></dc:subject><dc:creator></dc:creator><cp:keywords></cp:keywords><dc:description></dc:description><cp:lastModifiedBy></cp:lastModifiedBy><cp:revision>1</cp:revision></cp:coreProperties>';

const APP_PROPERTIES = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>InsynDoc</Application></Properties>';

const CUSTOM_PROPERTIES = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties"></Properties>';

function stripRevisionIds(xml: string): string {
  return xml
    .replace(/<w:rsids>[\s\S]*?<\/w:rsids>/g, '')
    .replace(/\s+w:rsid\w+="[^"]*"/g, '');
}

function stripPartReferences(xml: string, partName: string): string {
  const escaped = partName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return xml
    .replace(new RegExp(`<Override[^>]*PartName="/${escaped}"[^>]*/>`, 'g'), '')
    .replace(new RegExp(`<Relationship[^>]*Target="${escaped}"[^>]*/>`, 'g'), '');
}

export function sanitizeDocxMetadata(bytes: Uint8Array): Uint8Array {
  try {
    const parts = unzipSync(bytes);
    if (parts['docProps/core.xml']) parts['docProps/core.xml'] = strToU8(CORE_PROPERTIES);
    if (parts['docProps/app.xml']) parts['docProps/app.xml'] = strToU8(APP_PROPERTIES);
    if (parts['docProps/custom.xml']) parts['docProps/custom.xml'] = strToU8(CUSTOM_PROPERTIES);

    const thumbnails = Object.keys(parts).filter((name) => /^docProps\/thumbnail\./.test(name));
    for (const name of thumbnails) delete parts[name];
    for (const thumbnail of thumbnails) {
      const file = thumbnail.split('/').pop() ?? '';
      if (parts['[Content_Types].xml']) parts['[Content_Types].xml'] = strToU8(stripPartReferences(strFromU8(parts['[Content_Types].xml']), `docProps/${file}`));
      if (parts['_rels/.rels']) parts['_rels/.rels'] = strToU8(stripPartReferences(strFromU8(parts['_rels/.rels']), `docProps/${file}`));
    }

    for (const name of Object.keys(parts)) {
      if (!name.startsWith('word/') || !name.endsWith('.xml')) continue;
      parts[name] = strToU8(stripRevisionIds(strFromU8(parts[name])));
    }
    return zipSync(parts);
  } catch {
    return bytes.slice();
  }
}
