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
    const svg = el.querySelector('svg');
    return {
      x: +r.x.toFixed(1), y: +r.y.toFixed(1), w: +r.width.toFixed(1), h: +r.height.toFixed(1),
      label: el.getAttribute('aria-label') || el.getAttribute('title') || '',
      cls: (el.className || '').toString().slice(0, 60),
      svg: svg ? svg.outerHTML.slice(0, 1200) : null,
    };
  });
  return {
    toolbarButtons: grab('.docs-toolbar .docx-toolbar__button'),
    headerIcons: grab('.app-header .icon-button, .header-actions .icon-button, .header-actions button'),
    statusbar: grab('.status-bar button, .status-bar .icon-button'),
    findIcons: [],
  };
});
writeFileSync('D:/Github/InsynDoc/artifacts/ui/svg-extract.json', JSON.stringify(data, null, 1));
console.log('toolbar', data.toolbarButtons.length, 'header', data.headerIcons.length, 'status', data.statusbar.length);
await browser.close();
