import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium, expect } from '@playwright/test';

if (process.platform !== 'win32') throw new Error('The Native Autoformat Smoke Test Requires Windows.');

const server = createServer();
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
await new Promise((resolve) => server.close(resolve));
const profile = await mkdtemp(path.join(tmpdir(), 'InsynDoc-Autoformat-'));
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
  const normalSize = await page.getByRole('combobox', { name: 'Font Size', exact: true }).inputValue();
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await mkdir('artifacts/ui', { recursive: true });
  for (const marker of ['*', '-', '+', '1.', '1)', '#', '##', '###', '####', '#####', '######']) {
    await page.keyboard.type(`${marker} `);
    if (marker.startsWith('#')) await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText(`Heading ${marker.length}`);
    else await expect(page.locator('.docx-list-marker')).toHaveCount(1);
    if (marker === '##') await page.screenshot({ path: 'artifacts/ui/native-backspace-heading-before.png' });
    await page.keyboard.press('Backspace');
    assert.equal(await page.locator('.docx-page-content').first().textContent(), `${marker} `);
    await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText('Normal');
    await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue(normalSize);
    await expect(page.locator('.docx-list-marker')).toHaveCount(0);
    if (marker === '##') {
      await page.keyboard.type('Literal Text After Backspace');
      await page.screenshot({ path: 'artifacts/ui/native-backspace-shortcut-proof.png' });
    }
    await page.keyboard.press('Control+a');
    await page.keyboard.press('Backspace');
  }
  console.log('PASS: Native Backspace Restores All Eleven Shortcut Markers And Their Spaces');
  await page.keyboard.type('Normal Margin Reference');
  const normalMargin = await page.getByText('Normal Margin Reference', { exact: true }).boundingBox();
  await page.keyboard.press('Enter');
  await page.keyboard.type('* ');
  await expect(page.locator('.docx-list-marker')).toHaveCount(1);
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-list-marker')).toHaveCount(0);
  await expect(page.locator('.docx-page-content').first()).toContainText('* ');
  await page.keyboard.press('Control+Shift+z');
  await expect(page.locator('.docx-list-marker')).toHaveCount(1);
  await page.keyboard.type('First Bullet');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Second Bullet');
  await expect(page.locator('.docx-list-marker')).toHaveCount(2);
  await mkdir('artifacts/ui', { recursive: true });
  await page.screenshot({ path: 'artifacts/ui/native-autoformat-bullets.png' });
  console.log('PASS: Native Bullet Shortcut, Continued List, Undo, And Redo');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Backspace');
  await page.keyboard.type('Normal After Bullets');
  const afterBullets = await page.getByText('Normal After Bullets', { exact: true }).boundingBox();
  assert(Math.abs(afterBullets.x - normalMargin.x) < 0.5, 'Leaving Bullets Kept The List Indent.');
  await page.keyboard.press('Enter');
  await page.keyboard.type('1. First Numbered Item');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Second Numbered Item');
  await expect(page.locator('.docx-list-marker')).toHaveCount(4);
  await expect(page.locator('.docx-list-marker').nth(2)).toContainText('1.');
  await expect(page.locator('.docx-list-marker').nth(3)).toContainText('2.');
  await page.screenshot({ path: 'artifacts/ui/native-autoformat-lists.png' });
  console.log('PASS: Native Numbered Shortcut And Continued Numbering');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Backspace');
  await page.keyboard.type('Normal After Numbers');
  const afterNumbers = await page.getByText('Normal After Numbers', { exact: true }).boundingBox();
  assert(Math.abs(afterNumbers.x - normalMargin.x) < 0.5, 'Leaving Numbering Kept The List Indent.');
  await page.screenshot({ path: 'artifacts/ui/native-list-exit-margin-proof.png' });
  console.log('PASS: Native Backspace Leaves Empty Lists At The Normal Body Margin');
  await page.keyboard.press('Enter');
  await page.keyboard.type('## Automatic Heading');
  await expect(page.getByRole('button', { name: 'Automatic Heading', exact: true })).toBeVisible();
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue('22');
  await page.screenshot({ path: 'artifacts/ui/native-autoformat-verified.png' });
  console.log('PASS: Native Heading Shortcut, Heading Size, And Document Outline');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText('Normal');
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue(normalSize);
  await page.keyboard.press('Control+z');
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue('22');
  await page.keyboard.press('Control+Shift+z');
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue(normalSize);
  await page.keyboard.type('This Is Back To Normal Body Text');
  await page.screenshot({ path: 'artifacts/ui/native-heading-body-size-proof.png' });
  console.log('PASS: Native Heading Enter Restores Body Size With Undo And Redo');
  await page.evaluate(() => { void window.__TAURI_INTERNALS__.invoke('plugin:window|destroy'); });
  const code = await Promise.race([exited, delay(15_000, undefined, { ref: false }).then(() => { throw new Error('The Test App Did Not Exit.'); })]);
  assert.equal(code, 0, 'The Test App Did Not Exit Successfully.');
} finally {
  if (browser) await browser.close().catch(() => undefined);
  if (child.exitCode === null && !processError) child.kill();
}
