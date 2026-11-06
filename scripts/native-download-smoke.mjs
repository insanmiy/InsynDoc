import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium, expect } from '@playwright/test';
import { strFromU8, unzipSync } from 'fflate';

if (process.platform !== 'win32') throw new Error('Native Download Tests Require Windows.');
const executable = path.resolve(process.argv[2] ?? 'src-tauri/target/release/insyndoc.exe');
const profile = await mkdtemp(path.join(tmpdir(), 'InsynDoc-Downloads-'));
const output = path.resolve('artifacts/ui/downloads');
await mkdir(output, { recursive: true });
const server = createServer();
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
await new Promise((resolve) => server.close(resolve));
const child = spawn(executable, [], {
  windowsHide: true, stdio: 'ignore',
  env: { ...process.env, WEBVIEW2_USER_DATA_FOLDER: profile, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1` }
});
let processError;
child.on('error', (error) => { processError = error; });
const exited = new Promise((resolve) => child.on('exit', resolve));
let browser;
try {
  const endpoint = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 30000;
  let ready = false;
  while (Date.now() < deadline) {
    if (processError) throw processError;
    if (child.exitCode !== null) throw new Error('The Native App Exited Before Its Window Was Ready.');
    try { ready = (await fetch(`${endpoint}/json/version`)).ok; } catch { ready = false; }
    if (ready) break;
    await delay(150);
  }
  assert(ready, 'The Native Window Did Not Become Ready.');
  browser = await chromium.connectOverCDP(endpoint);
  const context = browser.contexts()[0];
  const page = context.pages()[0] ?? await context.waitForEvent('page');
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled({ timeout: 30000 });
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Download Format Verification');
  await page.keyboard.press('Enter');
  await page.keyboard.insertText('Unicode: R\u00e9sum\u00e9 \u03a9\n' + 'This Is Searchable Document Text For Export Verification.\n'.repeat(100) + 'END_OF_DOCUMENT');
  const formats = [
    ['docx', 'Microsoft Word (.docx)', 'docx'], ['pdf', 'PDF Document (.pdf)', 'pdf'],
    ['odt', 'OpenDocument Format (.odt)', 'odt'], ['txt', 'Plain Text (.txt)', 'txt'],
    ['rtf', 'Rich Text Format (.rtf)', 'rtf'], ['html', 'Web Page (.html, Zipped)', 'zip'],
    ['epub', 'EPUB Publication (.epub)', 'epub'], ['md', 'Markdown (.md)', 'md']
  ];
  for (const [format, label, extension] of formats) {
    await page.getByRole('button', { name: 'Download Document', exact: true }).click();
    if (format === 'docx') await page.screenshot({ path: 'artifacts/ui/native-download-formats-menu.png' });
    await page.getByRole('menuitem', { name: label, exact: true }).click();
    const destination = path.join(output, `Sample.${extension}`);
    // A New Filename Avoids An Overwrite Prompt During Repeated Verification Runs.
    const uniqueDestination = destination.replace(`.${extension}`, `-${Date.now()}.${extension}`);
    const helper = spawn('powershell.exe', ['-NoProfile', '-STA', '-WindowStyle', 'Hidden', '-ExecutionPolicy', 'Bypass', '-File', path.resolve('scripts/native-save-dialog.ps1'), '-ApplicationId', String(child.pid), '-Destination', uniqueDestination, '-DialogTitle', format === 'pdf' ? 'Download PDF' : 'Download Document'], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let helperError = '';
    helper.stderr.on('data', (data) => { helperError += data; });
    helper.on('error', (error) => { helperError += error.message; });
    const helperCode = await new Promise((resolve) => helper.on('exit', resolve));
    assert.equal(helperCode, 0, helperError || 'The Native Save Dialog Could Not Be Completed.');
    await expect(page.locator('.workspace-notice')).toContainText(`Downloaded ${path.basename(uniqueDestination)}`, { timeout: 30000 });
    await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
    await expect(page.locator('#pdf-export-root')).toHaveCount(0);
    await expect(page.locator('.document-name-button')).toHaveText('Untitled.docx');
    await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
    const bytes = new Uint8Array(await readFile(uniqueDestination));
    assert(bytes.length > 0, 'The Download Is Empty.');
    if (['docx', 'odt', 'epub', 'html'].includes(format)) {
      const entries = unzipSync(bytes);
      const body = format === 'docx' ? 'word/document.xml' : format === 'odt' ? 'content.xml' : format === 'epub' ? 'EPUB/content.xhtml' : 'index.html';
      assert(strFromU8(entries[body]).includes('END_OF_DOCUMENT'), 'The Download Omitted Document Content.');
    }
    if (format === 'pdf') assert(Buffer.from(bytes).subarray(0, 5).equals(Buffer.from('%PDF-')), 'The PDF Header Is Invalid.');
    if (['txt', 'rtf', 'md'].includes(format)) {
      const content = format === 'md' ? strFromU8(bytes).replace(/\\_/g, '_') : strFromU8(bytes);
      assert(content.includes('END_OF_DOCUMENT'));
    }
    console.log(`PASS: ${format.toUpperCase()} Download Saved Through The Real Native Dialog: ${uniqueDestination}`);
  }
  await page.evaluate(() => { void window.__TAURI_INTERNALS__.invoke('plugin:window|destroy'); });
  assert.equal(await Promise.race([exited, delay(15000, undefined, { ref: false }).then(() => { throw new Error('The Test App Did Not Exit.'); })]), 0);
} finally {
  if (browser) await browser.close().catch(() => undefined);
  if (child.exitCode === null && !processError) { child.kill(); await exited; }
}
