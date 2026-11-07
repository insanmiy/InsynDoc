import { chromium } from 'playwright';

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'load', timeout: 60000 });
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.waitForTimeout(2500);

async function openDialog(name) {
  await page.locator('.docx-menubar__trigger', { hasText: name }).click();
  await page.waitForTimeout(400);
}

// Paragraph dialog via Format menu
await openDialog('Format');
await page.getByText('Paragraph', { exact: true }).first().click();
await page.waitForTimeout(800);
await page.screenshot({ path: 'artifacts/ui/orig-paragraph-dialog.png' });
let html = await page.evaluate(() => document.querySelector('.docx-dialog-mount')?.innerHTML ?? 'none');
console.log('=== PARAGRAPH ===');
console.log(html.slice(0, 6000));
await page.keyboard.press('Escape');
await page.waitForTimeout(300);

// Date and time via Insert menu
await openDialog('Insert');
await page.getByText('Date And Time', { exact: true }).first().click();
await page.waitForTimeout(800);
await page.screenshot({ path: 'artifacts/ui/orig-datetime-dialog.png' });
html = await page.evaluate(() => {
  const d = document.querySelector('.writing-tools-modal, [role="dialog"]:last-of-type');
  return d ? d.innerHTML.slice(0, 6000) : 'none';
});
console.log('=== DATETIME ===');
console.log(html);
await browser.close();
