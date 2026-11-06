import { cp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';

const fonts = path.resolve('node_modules/@docx-editor.dev/fonts');
const licensesDirectory = 'public/licenses';
await rm(licensesDirectory, { recursive: true, force: true });
await mkdir(licensesDirectory, { recursive: true });
await cp(path.join(fonts, 'licenses'), path.join(licensesDirectory, 'fonts'), { recursive: true });
await cp('licenses/EigenPal-LICENSE.txt', path.join(licensesDirectory, 'EigenPal-LICENSE.txt'));
await cp('licenses/Inter-OFL.txt', path.join(licensesDirectory, 'Inter-OFL.txt'));
await cp('licenses/document-fonts', path.join(licensesDirectory, 'document-fonts'), { recursive: true });

const notices = ['# Third Party Notices', '', 'InsynDoc Uses The Following Production Packages. The Full License Texts Are Bundled In The licenses Directory.', ''];
function productionPackageDirectories() {
  const npmArguments = ['ls', '--omit=dev', '--all', '--parseable'];
  const result = process.platform === 'win32'
    ? spawnSync('cmd.exe', ['/d', '/s', '/c', 'npm', ...npmArguments], { encoding: 'utf8' })
    : spawnSync('npm', npmArguments, { encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Could Not List Production Packages: ${result.stderr || result.stdout}`);
  return result.stdout.split(/\r?\n/).map((line) => line.trim()).filter((line) => line.includes('node_modules')).sort();
}
async function collectPackage(directory) {
  const manifest = JSON.parse(await readFile(path.join(directory, 'package.json'), 'utf8'));
  if (manifest.name.startsWith('@types/')) return;
  notices.push(`* ${manifest.name} ${manifest.version}: ${manifest.license ?? 'See Package License'}`);
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
  ['Liberation Mono', 'SIL Open Font License 1.1 With Reserved Font Name Liberation', 'licenses/fonts/LICENSE-Liberation.txt'],
  ['Liberation Sans', 'SIL Open Font License 1.1 With Reserved Font Name Liberation', 'licenses/fonts/LICENSE-Liberation.txt'],
  ['Liberation Serif', 'SIL Open Font License 1.1 With Reserved Font Name Liberation', 'licenses/fonts/LICENSE-Liberation.txt'],
  ['Selawik', 'SIL Open Font License 1.1', 'licenses/fonts/OFL-Selawik.txt'],
  ['TeX Gyre Adventor', 'GUST Font License (LPPL 1.3c Or Later)', 'licenses/fonts/GUST-FONT-LICENSE.txt And licenses/fonts/LPPL-1.3c.txt'],
];
notices.push('', '## Bundled Interface Font', '', '* Inter: SIL Open Font License 1.1. See licenses/Inter-OFL.txt.');
notices.push('', '## Bundled Document Font Substitutes', '', ...packagedFontLicenses.map(([family, license, file]) => `* ${family}: ${license}. See ${file}.`));
notices.push('', '## Additional Bundled Document Fonts', '');
for (const family of documentFontFamilies) {
  const licenseFile = `${family.replaceAll(' ', '-')}-LICENSE.txt`;
  const licenseText = await readFile(path.join('licenses/document-fonts', licenseFile), 'utf8');
  const license = licenseText.includes('Apache License') ? 'Apache License 2.0' : 'SIL Open Font License 1.1';
  notices.push(`* ${family}: ${license}. See licenses/document-fonts/${licenseFile}`);
}
await writeFile('THIRD_PARTY_NOTICES.md', `${notices.join('\n')}\n`);
