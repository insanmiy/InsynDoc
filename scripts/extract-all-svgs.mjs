import { chromium } from 'playwright';
import { writeFileSync } from 'node:fs';
const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load' });
await page.waitForSelector('.app-shell');
await page.waitForTimeout(2500);
const data = await page.evaluate(() => {
  const grab = (sel) => [...document.querySelectorAll(sel)].map((el) => {
    const r = el.getBoundingClientRect();
    const svgs = [...el.querySelectorAll('svg')].map((s) => s.outerHTML);
    return {
      x: +r.x.toFixed(1), y: +r.y.toFixed(1), w: +r.width.toFixed(1), h: +r.height.toFixed(1),
      label: el.getAttribute('aria-label') || el.getAttribute('title') || '',
      cls: (el.className || '').toString().slice(0, 70),
      svgs,
    };
  });
  return {
    toolbar: grab('.docs-toolbar .docx-toolbar__button, .docs-toolbar .docx-toolbar__colorsplit-caret, .docs-toolbar .docx-toolbar__style-trigger, .docs-toolbar .docx-toolbar__font-family-trigger, .docs-toolbar .docx-toolbar__stepper-button, .docs-toolbar .docx-toolbar__stepper-value, .docs-toolbar .toolbar-menu-search'),
    toolbarEnd: grab('.toolbar-end, .toolbar-end *'),
    header: grab('.app-header button, .app-header svg, .document-name-button, .document-save-indicator, .app-logo img'),
    statusbar: grab('.status-bar *'),
    outline: grab('.document-outline *'),
  };
});
writeFileSync('D:/Github/InsynDoc/artifacts/ui/svg-full.json', JSON.stringify(data));
for (const [k, v] of Object.entries(data)) console.log(k, v.length);
await browser.close();
