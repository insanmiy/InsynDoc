import { chromium } from 'playwright';

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);
await page.locator('.docx-menubar__trigger', { hasText: 'Format' }).click();
await page.waitForTimeout(600);
await page.screenshot({ path: 'artifacts/ui/orig-format-menu.png' });
const items = await page.evaluate(() =>
  [...document.querySelectorAll('[role="menuitem"], [role="menu"] button, .docx-menu button, [class*="menu"] button')]
    .map((e) => e.textContent.trim().slice(0, 60))
    .filter(Boolean)
);
console.log(JSON.stringify(items, null, 1));
await browser.close();
