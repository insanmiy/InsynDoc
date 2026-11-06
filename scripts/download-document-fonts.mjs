import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { GOOGLE_FONT_CATALOG } from '@docx-editor.dev/fonts/google';

const families = JSON.parse(await readFile('src/document-font-families.json', 'utf8'));
const faces = GOOGLE_FONT_CATALOG.filter((face) => families.includes(face.family));
await mkdir('public/document-fonts', { recursive: true });
await mkdir('licenses/document-fonts', { recursive: true });

function primaryUrl(catalogUrl) {
  return catalogUrl.replace('https://cdn.jsdelivr.net/gh/google/fonts@', 'https://raw.githubusercontent.com/google/fonts/');
}

async function download(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(30000) });
  if (!response.ok) throw new Error(`Could Not Download ${url}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

const manifest = [];
for (let offset = 0; offset < faces.length; offset += 4) {
  const batch = await Promise.all(faces.slice(offset, offset + 4).map(async (face) => {
    const file = path.posix.basename(new URL(face.url).pathname);
    const target = path.join('public/document-fonts', file);
    let bytes;
    try { bytes = await readFile(target); } catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (!bytes || `sha256:${createHash('sha256').update(bytes).digest('hex')}` !== face.hash) bytes = await download(primaryUrl(face.url));
    const hash = `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
    if (bytes.byteLength !== face.byteLength || hash !== face.hash) throw new Error(`Font Verification Failed: ${file}`);
    await writeFile(target, bytes);
    return { ...face, file, source: primaryUrl(face.url) };
  }));
  manifest.push(...batch);
}

for (const family of families) {
  const face = faces.find((entry) => entry.family === family);
  if (!face) throw new Error(`Font Family Is Missing From The Catalog: ${family}`);
  const directory = primaryUrl(face.url).slice(0, primaryUrl(face.url).lastIndexOf('/') + 1).replace(/static\/$/, '');
  const licenseName = directory.includes('/apache/') ? 'LICENSE.txt' : 'OFL.txt';
  const license = await download(`${directory}${licenseName}`);
  await writeFile(`licenses/document-fonts/${family.replaceAll(' ', '-')}-LICENSE.txt`, license);
}
await writeFile('public/document-fonts/manifest.json', `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Downloaded And Verified ${faces.length} Faces Across ${families.length} Font Families.`);
