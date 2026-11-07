import { chromium } from 'playwright';
import { writeFileSync } from 'node:fs';

const BASE = 'http://127.0.0.1:1420';
const out = {};

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, deviceScaleFactor: 1 });
await context.addInitScript(() => {
  localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ theme: 'dark', rulerUnit: 'cm', wordGoal: 10 }));
});
const page = await context.newPage();
await page.goto(BASE, { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);

const measure = async (name, selector) => {
  const data = await page.evaluate((sel) => {
    const els = [...document.querySelectorAll(sel)];
    return els.slice(0, 80).map((el) => {
      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      return {
        cls: (el.className?.toString() || el.tagName).slice(0, 60),
        text: (el.textContent || '').trim().slice(0, 40),
        x: +r.x.toFixed(1), y: +r.y.toFixed(1), w: +r.width.toFixed(1), h: +r.height.toFixed(1),
        fs: cs.fontSize, fw: cs.fontWeight, color: cs.color, bg: cs.backgroundColor,
        border: cs.border, radius: cs.borderRadius, shadow: cs.boxShadow, pad: cs.padding, gap: cs.gap,
        opacity: cs.opacity, display: cs.display,
      };
    });
  }, selector);
  out[name] = data;
};

// Header
await measure('header', '.app-header, .app-header *, .header-actions, .header-actions *, .app-logo, .document-title-row, .document-title-row *');
// Toolbar
await measure('toolbar', '.toolbar-shell, .toolbar-shell *, .docs-toolbar, .docs-toolbar *');
// Status bar
await measure('statusbar', '.status-bar, .status-bar *');
// Outline
await measure('outline', '.document-outline, .document-outline *');
// Ruler
await measure('ruler', '.ruler-wrap, .ruler-wrap *');

// Settings modal
await page.keyboard.press('Control+,');
await page.waitForTimeout(800);
await measure('settings', '.modal, .modal *, .settings-modal *');
await page.keyboard.press('Escape');
await page.waitForTimeout(400);

// Shortcuts modal
await page.keyboard.press('Control+/');
await page.waitForTimeout(800);
await measure('shortcuts', '.modal, .modal *, .shortcut-list, .shortcut-list *');
await page.keyboard.press('Escape');
await page.waitForTimeout(400);

// Menu search
await page.evaluate(() => window.dispatchEvent(new KeyboardEvent('keydown', { key: '/', code: 'Slash', altKey: true, bubbles: true })));
await page.waitForTimeout(800);
await measure('menuSearch', '.modal, .modal *, .menu-search-modal, .menu-search-modal *');
await page.keyboard.press('Escape');
await page.waitForTimeout(400);

// Find with replace + text
await page.mouse.click(700, 400);
await page.waitForTimeout(300);
await page.keyboard.type('Find This Text');
await page.keyboard.press('Enter');
await page.keyboard.type('Find More Text');
await page.waitForTimeout(800);
await page.keyboard.press('Control+Shift+i');
await page.waitForTimeout(400);
await page.getByLabel('Find Text').fill('Find');
await page.waitForTimeout(800);
await measure('find', '.find-panel, .find-panel *');

writeFileSync('D:/Github/InsynDoc/artifacts/ui/dom-measure.json', JSON.stringify(out, null, 1));
console.log('done', Object.keys(out).map(k => `${k}:${out[k].length}`).join(' '));
await browser.close();
