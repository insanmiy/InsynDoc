import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium, expect } from '@playwright/test';

if (process.platform !== 'win32') throw new Error('The Native Spelling Smoke Test Requires Windows.');

const server = createServer();
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
await new Promise((resolve) => server.close(resolve));
const profile = await mkdtemp(path.join(tmpdir(), 'InsynDoc-Spelling-'));
const executable = path.resolve(process.argv[2] ?? 'src-tauri/target/release/insyndoc.exe');
const child = spawn(executable, [], {
  windowsHide: true,
  stdio: 'ignore',
  env: {
    ...process.env,
    WEBVIEW2_USER_DATA_FOLDER: profile,
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
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled({ timeout: 30_000 });
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await expect(popup).toContainText('No Spelling Issues Found', { timeout: 15_000 });
  console.log('PASS: Packaged Worker Loaded Its Local Dictionary Under The Desktop Security Policy');
  await page.keyboard.press('Escape');
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('This is a speling test.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  await expect(popup.getByRole('button', { name: 'spelling', exact: true })).toBeVisible();
  await mkdir('artifacts/ui', { recursive: true });
  await page.screenshot({ path: 'artifacts/ui/native-spelling-verified.png' });
  await popup.getByRole('button', { name: 'spelling', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toContainText('This is a spelling test.');
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').first()).toContainText('speling');
  console.log('PASS: Native Spelling Underlines, Suggestions, Correction, And Undo');
  await page.keyboard.press('Control+a');
  await page.keyboard.type('I want to me there.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  await expect(popup).toContainText('Grammar Suggestion');
  await expect(popup.getByRole('button', { name: 'be', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('.docx-page-content').first()).toContainText('I want to be there.');
  console.log('PASS: Native Offline Sentence Context And Grammar Correction');
  await page.evaluate(() => { void window.__TAURI_INTERNALS__.invoke('plugin:window|destroy'); });
  const code = await Promise.race([exited, delay(15_000, undefined, { ref: false }).then(() => { throw new Error('The Test App Did Not Exit.'); })]);
  assert.equal(code, 0, 'The Test App Did Not Exit Successfully.');
} finally {
  if (browser) await browser.close().catch(() => undefined);
  if (child.exitCode === null && !processError) child.kill();
}
