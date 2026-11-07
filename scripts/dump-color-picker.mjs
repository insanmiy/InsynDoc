import { chromium } from 'playwright';
import fs from 'node:fs';

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2000);
await page.locator('.docx-toolbar__colorsplit-caret').first().click();
await page.waitForTimeout(500);
const html = await page.evaluate(() => document.querySelector('.docx-toolbar__swatch-popup')?.outerHTML ?? 'NONE');
fs.writeFileSync('artifacts/ui/orig-color-popup.html', html);
const simplified = html
  .replace(/<button[^>]*class="docx-toolbar__swatch[^"]*"[^>]*data-value="([^"]+)"[^>]*><\/button>/g, '[$1]')
  .replace(/<button[^>]*class="docx-toolbar__swatch[^"]*"[^>]*><\/button>/g, '[swatch]')
  .replace(/<[^>]+>/g, '|')
  .replace(/\|+/g, ' | ');
console.log(simplified.slice(0, 6000));
await browser.close();
