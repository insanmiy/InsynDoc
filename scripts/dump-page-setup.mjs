import { chromium } from 'playwright';

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);
await page.locator('.docx-menubar__trigger', { hasText: 'File' }).click();
await page.waitForTimeout(500);
await page.screenshot({ path: 'artifacts/ui/orig-file-menu.png' });
await page.getByText('Page Setup', { exact: true }).first().click();
await page.waitForTimeout(800);
await page.screenshot({ path: 'artifacts/ui/orig-page-setup.png' });
const html = await page.evaluate(() => {
  const dialogs = document.querySelectorAll('[role="dialog"], .docx-dialog, [class*="dialog"], [class*="modal"]');
  return Array.from(dialogs).map((d) => d.outerHTML.slice(0, 8000)).join('\n---\n');
});
console.log(html || 'no dialog found');
await browser.close();
