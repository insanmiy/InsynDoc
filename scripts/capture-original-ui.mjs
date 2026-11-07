import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const BASE = 'http://127.0.0.1:1420';
const OUT = new URL('../artifacts/ui/', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1');
mkdirSync(OUT, { recursive: true });

const STATES = [
  { name: 'dark-editor', theme: 'dark' },
  { name: 'dark-settings', theme: 'dark', prefs: { rulerUnit: 'cm', wordGoal: 10 }, action: 'settings' },
  { name: 'light-editor', theme: 'light' },
  { name: 'dark-shortcuts', theme: 'dark', action: 'shortcuts' },
  { name: 'dark-menu-search', theme: 'dark', action: 'menuSearch' },
  { name: 'dark-find', theme: 'dark', action: 'find' },
];

const browser = await chromium.launch();
for (const state of STATES) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, deviceScaleFactor: 1 });
  await context.addInitScript((prefs) => {
    localStorage.setItem('insyndoc.preferences.v1', JSON.stringify(prefs));
  }, { theme: state.theme, ...(state.prefs ?? {}) });
  const page = await context.newPage();
  await page.goto(BASE, { waitUntil: 'load', timeout: 60000 });
  await page.waitForSelector('.app-shell', { timeout: 30000 });
  await page.waitForTimeout(2500);
  if (state.action === 'settings') {
    await page.keyboard.press('Control+,');
  } else if (state.action === 'shortcuts') {
    await page.keyboard.press('Control+/');
  } else if (state.action === 'menuSearch') {
    await page.evaluate(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '/', code: 'Slash', altKey: true, bubbles: true }));
    });
  } else if (state.action === 'find') {
    await page.mouse.click(700, 400);
    await page.waitForTimeout(300);
    await page.keyboard.type('Find This Text');
    await page.keyboard.press('Enter');
    await page.keyboard.type('Find More Text');
    await page.waitForTimeout(800);
    await page.keyboard.press('Control+Shift+i');
    await page.waitForTimeout(400);
    const input = page.getByLabel('Find Text');
    await input.fill('Find');
    await page.waitForTimeout(800);
  }
  await page.waitForTimeout(600);
  const path = join(OUT, `orig-${state.name}-gemini1.png`);
  await page.screenshot({ path });
  console.log('saved', path);
  await context.close();
}
await browser.close();
