import { chromium } from 'playwright';

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);
await page.locator('.docx-menubar__trigger', { hasText: 'Format' }).click();
await page.waitForTimeout(300);
await page.locator('[role="menuitem"]', { hasText: 'Line Spacing Options' }).click();
await page.waitForTimeout(500);
await page.screenshot({ path: 'artifacts/ui/orig-paragraph-dialog.png' });
const dialogs = await page.evaluate(() =>
  [...document.querySelectorAll('[role="dialog"], .docx-dialog, [class*="modal"]')]
    .filter((el) => el.getClientRects().length > 0)
    .map((el) => el.outerHTML.slice(0, 15000))
);
console.log(dialogs.join('\n=====\n').slice(0, 12000) || 'NO DIALOG');
await browser.close();
