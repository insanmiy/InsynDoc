import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export function releaseNotesFor(changelog, tag, packageVersion) {
  if (tag !== `v${packageVersion}`) {
    throw new Error('The release tag must match the version in package.json.');
  }
  const lines = changelog.replace(/\r\n?/g, '\n').split('\n');
  const heading = `## [${packageVersion}]`;
  const starts = lines.flatMap((line, index) => line === heading || line.startsWith(`${heading} - `) ? [index] : []);
  if (starts.length !== 1) {
    throw new Error(`Add exactly one ${heading} section to CHANGELOG.md and move the release entries out of Unreleased.`);
  }
  const start = starts[0] + 1;
  const followingHeading = lines.findIndex((line, index) => index >= start && line.startsWith('## '));
  const notes = lines.slice(start, followingHeading === -1 ? undefined : followingHeading).join('\n').trim();
  if (!/^[-*] \S/m.test(notes)) {
    throw new Error(`Add release entries to the ${heading} section in CHANGELOG.md.`);
  }
  return `${notes}\n`;
}

async function main() {
  const [tag, outputPath = 'artifacts/release-notes.md'] = process.argv.slice(2);
  const manifest = JSON.parse(await readFile('package.json', 'utf8'));
  const notes = releaseNotesFor(await readFile('CHANGELOG.md', 'utf8'), tag, manifest.version);
  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, notes, 'utf8');
  console.log(`Wrote release notes for ${tag} to ${outputPath}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
