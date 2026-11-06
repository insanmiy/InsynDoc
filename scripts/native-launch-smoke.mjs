import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readFile, unlink, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium, expect } from '@playwright/test';
import { strToU8, zipSync } from 'fflate';

if (process.platform !== 'win32') throw new Error('The Native Launch Smoke Test Requires Windows.');
if (!process.env.APPDATA) throw new Error('The Windows Application Data Directory Is Unavailable.');

function runPowerShell(command) {
  return new Promise((resolve, reject) => {
    const helper = spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-WindowStyle', 'Hidden', '-Command', command], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let output = '';
    let error = '';
    helper.stdout.setEncoding('utf8').on('data', (text) => { output += text; });
    helper.stderr.setEncoding('utf8').on('data', (text) => { error += text; });
    helper.on('error', reject);
    helper.on('exit', (code) => code === 0 ? resolve(output.trim()) : reject(new Error(error || 'The Windows Test Helper Failed.')));
  });
}

async function readOptional(filename) {
  try { return await readFile(filename); }
  catch (error) { if (error.code === 'ENOENT') return null; throw error; }
}

function recentPaths(bytes) {
  try {
    const entries = JSON.parse(bytes?.toString('utf8') ?? '[]');
    return Array.isArray(entries) ? entries.filter((entry) => typeof entry === 'string' && /\.docx$/i.test(entry)).slice(0, 10) : [];
  } catch { return []; }
}

const executable = path.resolve(process.argv[2] ?? 'src-tauri/target/release/insyndoc.exe');
const runningApps = await runPowerShell('@(Get-Process -Name insyndoc -ErrorAction SilentlyContinue).Count');
assert.equal(runningApps, '0', 'Close Other InsynDoc Windows Before Running This Test.');
const temporaryDirectory = await mkdtemp(path.join(tmpdir(), 'InsynDoc Native Launch '));
const documentPath = path.join(temporaryDirectory, 'Downloaded R\u00e9sum\u00e9 Final.docx');
const documentName = path.basename(documentPath);
const content = 'Downloaded Document Opens In InsynDoc';
const fixture = zipSync({
  '[Content_Types].xml': strToU8('<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>'),
  '_rels/.rels': strToU8('<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>'),
  'word/document.xml': strToU8(`<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>${content}</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>`)
});
await writeFile(documentPath, fixture);
await writeFile(`${documentPath}:Zone.Identifier`, '[ZoneTransfer]\r\nZoneId=3\r\n');
const historyPath = path.join(process.env.APPDATA, 'com.insyndoc.editor', 'recent-documents.json');
const originalHistory = await readOptional(historyPath);
if (originalHistory) await writeFile(path.join(temporaryDirectory, 'recent-documents.backup.json'), originalHistory);
const expectedHistory = Buffer.from(JSON.stringify([documentPath, ...recentPaths(originalHistory).filter((entry) => entry !== documentPath)].slice(0, 10)));
const server = createServer();
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
await new Promise((resolve) => server.close(resolve));
const child = spawn(executable, [documentPath], {
  windowsHide: true,
  stdio: 'ignore',
  env: {
    ...process.env,
    WEBVIEW2_USER_DATA_FOLDER: path.join(temporaryDirectory, 'WebViewProfile'),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`
  }
});
let processError;
child.on('error', (error) => { processError = error; });
const exited = new Promise((resolve) => child.on('exit', (code) => resolve(code)));
let browser;
try {
  const endpoint = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 30_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (processError) throw processError;
    if (child.exitCode !== null) throw new Error('The Native App Exited Before The WebView Was Ready.');
    try { ready = (await fetch(`${endpoint}/json/version`)).ok; } catch { ready = false; }
    if (ready) break;
    await delay(150);
  }
  assert(ready, 'The Packaged WebView Did Not Become Ready.');
  browser = await chromium.connectOverCDP(endpoint);
  const context = browser.contexts()[0];
  const page = context.pages()[0] ?? await context.waitForEvent('page');
  const externalRequests = [];
  context.on('request', (request) => {
    const url = new URL(request.url());
    if (/^https?:$/.test(url.protocol) && !['tauri.localhost', 'ipc.localhost', '127.0.0.1', 'localhost'].includes(url.hostname)) externalRequests.push(request.url());
  });
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator('.document-name-button')).toContainText(documentName);
  await expect(page.locator('.docx-page-content').first()).toContainText(content);
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await mkdir('artifacts/ui', { recursive: true });
  await page.screenshot({ path: 'artifacts/ui/native-docx-launch.png' });
  console.log('PASS: Native Startup Opens A Downloaded DOCX With Spaces And Unicode In Its Path');
  await page.keyboard.press('Control+n');
  await expect(page.locator('.document-name-button')).toContainText('Untitled.docx');
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('fuckk');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await expect(popup.getByRole('button', { name: 'fuck', exact: true })).toBeVisible({ timeout: 15_000 });
  await page.screenshot({ path: 'artifacts/ui/native-unrestricted-spelling.png' });
  await popup.getByRole('button', { name: 'fuck', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toHaveText('fuck');
  console.log('PASS: Native Spelling Suggestions Do Not Suppress Profanity');
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Backspace');
  await expect(page.locator('.docx-page-content').first()).toHaveText('');
  await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
  assert(Number.isSafeInteger(child.pid), 'The Native Test Process Has No Valid PID.');
  const closeRequested = await runPowerShell(`[Diagnostics.Process]::GetProcessById(${child.pid}).CloseMainWindow()`);
  assert.equal(closeRequested, 'True', 'Windows Did Not Deliver The Close Request.');
  const code = await Promise.race([
    exited,
    delay(15_000, undefined, { ref: false }).then(async () => {
      const confirmationShown = await page.getByRole('dialog', { name: 'Save Your Changes?', exact: true }).count().catch(() => 0);
      throw new Error(confirmationShown ? 'An Empty Document Incorrectly Requested Save Confirmation.' : 'The Empty Native App Did Not Exit.');
    })
  ]);
  assert.equal(code, 0, 'The Test App Did Not Exit Successfully.');
  assert.deepEqual(await readFile(documentPath), Buffer.from(fixture), 'The Downloaded Original Was Modified.');
  assert.deepEqual(externalRequests, [], 'Native Document Editing Made External Network Requests.');
  console.log('PASS: Clearing The Page Allows A Real Windows Close Request Without Save Confirmation');
  console.log(`Test Artifacts: ${temporaryDirectory}`);
} finally {
  if (browser) await browser.close().catch(() => undefined);
  if (child.exitCode === null && !processError) {
    child.kill();
    await exited;
  }
  const currentHistory = await readOptional(historyPath);
  if (currentHistory?.equals(expectedHistory)) {
    if (originalHistory) await writeFile(historyPath, originalHistory);
    else await unlink(historyPath);
  } else if (currentHistory && !currentHistory.equals(originalHistory ?? Buffer.alloc(0))) {
    const currentPaths = recentPaths(currentHistory);
    if (currentPaths.includes(documentPath)) await writeFile(historyPath, JSON.stringify(currentPaths.filter((entry) => entry !== documentPath)));
  }
}
