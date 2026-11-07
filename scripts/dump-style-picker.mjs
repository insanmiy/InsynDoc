import { chromium } from 'playwright';

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);
await page.click('[aria-label="Paragraph Style"]');
await page.waitForTimeout(500);
const options = await page.$$eval('[role="option"]', (els) =>
  els.map((el) => ({
    text: el.textContent.trim(),
    fontSize: getComputedStyle(el).fontSize,
    fontWeight: getComputedStyle(el).fontWeight,
  }))
);
console.log(JSON.stringify(options, null, 2));
await page.screenshot({ path: 'artifacts/ui/orig-style-picker.png' });
await browser.close();
