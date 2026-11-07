import { chromium } from 'playwright';
const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load' });
await page.waitForSelector('.app-shell');
await page.waitForTimeout(2500);
const data = await page.evaluate(() => {
  const els = [...document.querySelectorAll('.docs-toolbar .docx-toolbar__button, .docs-toolbar .docx-toolbar__separator, .docs-toolbar .docx-toolbar__group, .toolbar-end, .toolbar-end *')];
  return els.map((el) => {
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return [r.x.toFixed(0), r.y.toFixed(0), r.width.toFixed(0), r.height.toFixed(0), (el.className || '').toString().slice(0, 55), (el.textContent || '').trim().slice(0, 24), cs.color, cs.opacity].join(' | ');
  });
});
console.log(data.join('\n'));
await browser.close();
