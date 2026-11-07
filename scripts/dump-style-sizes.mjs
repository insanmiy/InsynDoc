import { chromium } from 'playwright';

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);
await page.mouse.click(700, 400);
await page.waitForTimeout(300);
await page.keyboard.type('Style Probe Text');
await page.keyboard.press('Control+a');
await page.waitForTimeout(300);

await page.click('[aria-label="Paragraph Style"]');
await page.waitForTimeout(400);
const count = await page.locator('[role="option"]').count();
console.log('options:', count);

const measure = () => page.evaluate(() => {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  let node;
  while ((node = walker.nextNode())) {
    if (node.textContent.includes('Style Probe Text')) {
      const cs = getComputedStyle(node.parentElement);
      return { fontSize: cs.fontSize, fontWeight: cs.fontWeight, fontStyle: cs.fontStyle, color: cs.color };
    }
  }
  return null;
});

for (let i = 0; i < count; i++) {
  const label = await page.locator('[role="option"]').nth(i).textContent();
  await page.evaluate((idx) => {
    document.querySelectorAll('[role="option"]')[idx].click();
  }, i);
  await page.waitForTimeout(400);
  console.log(label.trim(), JSON.stringify(await measure()));
  await page.click('[aria-label="Paragraph Style"]');
  await page.waitForTimeout(300);
}
await browser.close();
