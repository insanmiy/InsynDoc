import { cp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import path from 'node:path';

const fonts = path.resolve('node_modules/@docx-editor.dev/fonts');
const licensesDirectory = 'public/licenses';
const noticesFile = 'THIRD_PARTY_NOTICES.md';
const cacheFile = 'node_modules/.cache/insyndoc-prepare-assets.sha256';

async function hashPath(hash, filePath) {
  hash.update(filePath.replaceAll(path.sep, '/'));
  hash.update('\0');
  try {
    hash.update(await readFile(filePath));
  } catch {
    hash.update('missing');
  }
  hash.update('\0');
}

async function hashFile(hash, filePath) {
  await hashPath(hash, filePath);
}

async function hashDirectory(hash, directory) {
  hash.update(directory);
  hash.update('\0');
  let files = [];
  try {
    files = await readdir(directory, { recursive: true });
  } catch {
    hash.update('missing-directory');
    return;
  }
  files = files.filter((name) => !name.includes('node_modules')).sort();
  hash.update(String(files.length));
  hash.update('\0');
  for (const name of files) {
    const fullPath = path.join(directory, name);
    try {
      const entries = await readdir(fullPath);
      void entries;
      hash.update('directory:');
      hash.update(name);
      hash.update('\0');
    } catch {
      await hashPath(hash, fullPath);
    }
  }
}

async function computeInputsHash() {
  const hash = createHash('sha256');
  await hashFile(hash, 'scripts/prepare-assets.mjs');
  await hashFile(hash, 'package-lock.json');
  await hashFile(hash, 'node_modules/.package-lock.json');
  await hashFile(hash, 'src/document-font-families.json');
  await hashDirectory(hash, 'licenses');
  await hashDirectory(hash, path.join(fonts, 'licenses'));
  return hash.digest('hex');
}

const inputsHash = await computeInputsHash();
let cachedHash = null;
try {
  cachedHash = (await readFile(cacheFile, 'utf8')).trim();
} catch {
  cachedHash = null;
}
const licensesExist = existsSync(licensesDirectory) && (await readdir(licensesDirectory)).length > 0;
if (cachedHash === inputsHash && existsSync(noticesFile) && licensesExist) {
  process.exit(0);
}

await rm(licensesDirectory, { recursive: true, force: true });
await mkdir(licensesDirectory, { recursive: true });
await cp(path.join(fonts, 'licenses'), path.join(licensesDirectory, 'fonts'), { recursive: true });
await cp('licenses/EigenPal-LICENSE.txt', path.join(licensesDirectory, 'EigenPal-LICENSE.txt'));
await cp('licenses/Inter-OFL.txt', path.join(licensesDirectory, 'Inter-OFL.txt'));
await cp('licenses/document-fonts', path.join(licensesDirectory, 'document-fonts'), { recursive: true });

const notices = ['# Third party notices', '', 'InsynDoc uses the following production packages. The full license texts are bundled in the licenses directory.', ''];
function productionPackageDirectories() {
  const npmArguments = ['ls', '--omit=dev', '--all', '--parseable'];
  const result = process.platform === 'win32'
    ? spawnSync('cmd.exe', ['/d', '/s', '/c', 'npm', ...npmArguments], { encoding: 'utf8' })
    : spawnSync('npm', npmArguments, { encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Could not list production packages: ${result.stderr || result.stdout}`);
  return result.stdout.split(/\r?\n/).map((line) => line.trim()).filter((line) => line.includes('node_modules')).sort();
}
async function collectPackage(directory) {
  const manifest = JSON.parse(await readFile(path.join(directory, 'package.json'), 'utf8'));
  if (manifest.name.startsWith('@types/')) return;
  notices.push(`* ${manifest.name} ${manifest.version}: ${manifest.license ?? 'See package license'}`);
  const licenseFiles = (await readdir(directory)).filter((name) => /^(license|copying|notice|third.party.notices)(\.|$)/i.test(name));
  for (const name of licenseFiles) {
    const target = `${manifest.name.replaceAll('/', '_')}_${name}`;
    await cp(path.join(directory, name), path.join(licensesDirectory, target));
  }
}
for (const directory of productionPackageDirectories()) await collectPackage(directory);
const documentFontFamilies = JSON.parse(await readFile('src/document-font-families.json', 'utf8'));
const packagedFontLicenses = [
  ['Caladea', 'SIL Open Font License 1.1', 'licenses/fonts/OFL-Caladea.txt'],
  ['Carlito', 'SIL Open Font License 1.1', 'licenses/fonts/OFL-Carlito.txt'],
  ['Liberation Mono', 'SIL Open Font License 1.1 with reserved font name Liberation', 'licenses/fonts/LICENSE-Liberation.txt'],
  ['Liberation Sans', 'SIL Open Font License 1.1 with reserved font name Liberation', 'licenses/fonts/LICENSE-Liberation.txt'],
  ['Liberation Serif', 'SIL Open Font License 1.1 with reserved font name Liberation', 'licenses/fonts/LICENSE-Liberation.txt'],
  ['Selawik', 'SIL Open Font License 1.1', 'licenses/fonts/OFL-Selawik.txt'],
  ['TeX Gyre Adventor', 'GUST Font License (LPPL 1.3c or later)', 'licenses/fonts/GUST-FONT-LICENSE.txt and licenses/fonts/LPPL-1.3c.txt'],
];
notices.push('', '## Bundled interface font', '', '* Inter: SIL Open Font License 1.1. See licenses/Inter-OFL.txt.');
notices.push('', '## Bundled document font substitutes', '', ...packagedFontLicenses.map(([family, license, file]) => `* ${family}: ${license}. See ${file}.`));
notices.push('', '## Additional bundled document fonts', '');
for (const family of documentFontFamilies) {
  const licenseFile = `${family.replaceAll(' ', '-')}-LICENSE.txt`;
  const licenseText = await readFile(path.join('licenses/document-fonts', licenseFile), 'utf8');
  const license = licenseText.includes('Apache License') ? 'Apache License 2.0' : 'SIL Open Font License 1.1';
  notices.push(`* ${family}: ${license}. See licenses/document-fonts/${licenseFile}`);
}
await writeFile(noticesFile, `${notices.join('\n')}\n`);

await mkdir(path.dirname(cacheFile), { recursive: true });
await writeFile(cacheFile, `${inputsHash}\n`);
