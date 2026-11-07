import { chromium } from 'playwright';

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2000);
const html = await page.evaluate(() => {
  const els = [...document.querySelectorAll('.docx-toolbar button[aria-label="Font Color"]')];
  return els.map((e) => e.parentElement.outerHTML.slice(0, 1500)).join('\n=====\n');
});
console.log(html);
await browser.close();
