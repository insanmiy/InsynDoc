import { chromium } from 'playwright';

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);

for (const name of ['File', 'Edit', 'View', 'Insert', 'Format', 'Tools', 'Help']) {
  await page.locator('.docx-menubar__trigger', { hasText: name }).first().click();
  await page.waitForTimeout(500);
  const items = await page.evaluate(() => {
    const menus = [...document.querySelectorAll('[role="menu"], [class*="dropdown"], [class*="popup"]')];
    const visible = menus.filter((m) => m.offsetParent !== null);
    const top = visible[visible.length - 1];
    if (!top) return [];
    return [...top.querySelectorAll('button, [role="menuitem"], [role="option"]')]
      .map((e) => e.textContent.trim().replace(/\s+/g, ' '))
      .filter(Boolean);
  });
  console.log(`=== ${name} ===`);
  console.log(JSON.stringify(items));
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
}
await browser.close();
