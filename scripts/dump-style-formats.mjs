import { chromium } from 'playwright';

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);

const editable = page.locator('[contenteditable="true"], .docx-page').first();
await editable.click();
await page.keyboard.type('Sample Text');

// Collect style names once.
await page.locator('.docx-toolbar__style-trigger').click();
await page.waitForTimeout(300);
const names = await page.locator('.paragraph-style-menu [role="option"]').allTextContents();
console.error('styles:', JSON.stringify(names));
await page.keyboard.press('Escape');

const results = {};
for (const name of names.map((n) => n.trim())) {
  await page.locator('.docx-toolbar__style-trigger').click();
  await page.waitForTimeout(250);
  await page.locator('.paragraph-style-menu [role="option"]', { hasText: name }).first().click();
  await page.waitForTimeout(300);
  const style = await page.evaluate(() => {
    const el = document.querySelector('.docx-page [data-paragraph-id], .docx-page p, .docx-page [class*="para"]');
    if (!el) return null;
    const cs = getComputedStyle(el);
    return {
      fontSize: cs.fontSize,
      color: cs.color,
      fontWeight: cs.fontWeight,
      fontStyle: cs.fontStyle,
    };
  });
  results[name] = style;
}
console.log(JSON.stringify(results, null, 1));
await browser.close();
