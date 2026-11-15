import { expect, test, type Page } from '@playwright/test';

async function openMisspelling(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: true, updateNotifications: false }));
  });
  await page.goto('http://127.0.0.1:1420');
  const editor = page.getByRole('textbox', { name: 'Document content' });
  await expect(editor).toBeVisible();
  await editor.focus();
  await page.keyboard.insertText('profromace');
  const mark = page.locator('.spelling-mark').first();
  await expect(mark).toBeAttached({ timeout: 20_000 });
  const bounds = await mark.boundingBox();
  if (!bounds) throw new Error('The Spelling Mark Has No Bounds.');
  return { editor, x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 };
}

test('Hover Opens Suggestions Without Moving The Caret And Allows Correction', async ({ page }) => {
  const grammarRequests: string[] = [];
  page.on('request', (request) => { if (/harper.*\.wasm/.test(request.url())) grammarRequests.push(request.url()); });
  const { editor, x, y } = await openMisspelling(page);
  expect(grammarRequests).toHaveLength(0);
  const selection = await page.evaluate(() => window.getSelection()?.toString());
  await page.mouse.move(x, y);
  const popup = page.getByRole('dialog', { name: /^spelling suggestions$/i });
  await expect(popup).toBeVisible();
  await expect(popup.getByRole('button', { name: 'performance', exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(selection);
  await popup.getByRole('button', { name: 'performance', exact: true }).hover();
  await expect(popup).toBeVisible();
  await popup.getByRole('button', { name: 'performance', exact: true }).click();
  await expect(editor).toContainText('performance');
  await expect(popup).not.toBeVisible();
});

test('Click Places The Caret And Keyboard Review Still Opens Suggestions', async ({ page }) => {
  const { x, y } = await openMisspelling(page);
  await page.mouse.click(x, y);
  const popup = page.getByRole('dialog', { name: /^spelling suggestions$/i });
  await page.waitForTimeout(350);
  await expect(popup).not.toBeVisible();
  await page.keyboard.press('Control+Alt+x');
  await expect(popup).toBeVisible();
  await expect(popup.getByRole('button', { name: 'performance', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(popup).not.toBeVisible();
});

test('Moving Away Dismisses A Hover Menu', async ({ page }) => {
  const { x, y } = await openMisspelling(page);
  await page.mouse.move(x, y);
  const popup = page.getByRole('dialog', { name: /^spelling suggestions$/i });
  await expect(popup).toBeVisible();
  await page.mouse.move(5, 5);
  await expect(popup).not.toBeVisible();
});

test('Escape Closes The Download Menu After Hovering A Spelling Mark', async ({ page }) => {
  const { x, y } = await openMisspelling(page);
  await page.mouse.move(x, y);
  await expect(page.locator('.spelling-popover')).toBeVisible();
  const trigger = page.getByRole('button', { name: 'Download document' });
  await trigger.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('menu', { name: 'Download formats' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('menu', { name: 'Download formats' })).not.toBeVisible();
  await expect(trigger).toBeFocused();
});

test('Hover Suggestions Stay Open While Crossing The Gap To The Popup', async ({ page }) => {
  const { x, y } = await openMisspelling(page);
  await page.mouse.move(x, y);
  const popup = page.locator('.spelling-popover');
  await expect(popup).toBeVisible();
  const bounds = await popup.boundingBox();
  const mark = await page.locator('.spelling-mark').first().boundingBox();
  if (!bounds || !mark) throw new Error('The Spelling Popup Has No Bounds.');
  await page.mouse.move(x, (mark.y + mark.height + bounds.y) / 2);
  await page.waitForTimeout(400);
  await expect(popup).toBeVisible();
  await popup.getByRole('button', { name: 'performance', exact: true }).click();
  await expect(page.getByRole('textbox', { name: 'Document content' })).toContainText('performance');
});

test.describe('Touch Spelling', () => {
  test.use({ hasTouch: true });

  test('Tapping A Spelling Mark Opens Its Suggestions', async ({ page }) => {
    const { x, y } = await openMisspelling(page);
    await page.touchscreen.tap(x, y);
    const popup = page.getByRole('dialog', { name: /^spelling suggestions$/i });
    await expect(popup).toBeVisible();
    await popup.getByRole('button', { name: 'performance', exact: true }).tap();
    await expect(page.getByRole('textbox', { name: 'Document content' })).toContainText('performance');
  });
});

test('Learned Corrections Stay Flagged Until Explicitly Added To The Dictionary', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({
      checkSpelling: true, updateNotifications: false,
      spellingCorrections: [{ source: 'profromace', replacement: 'zorbulate', count: 1 }]
    }));
  });
  await page.goto('http://127.0.0.1:1420');
  const editor = page.getByRole('textbox', { name: 'Document content' });
  await expect(editor).toBeVisible();
  await editor.focus();
  await page.keyboard.insertText('zorbulate');
  await expect(page.locator('.spelling-mark').first()).toBeAttached({ timeout: 20_000 });
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: /^spelling suggestions$/i });
  await expect(popup).toBeVisible();
  await popup.getByRole('button', { name: 'Add to dictionary', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  const preferences = await page.evaluate(() => JSON.parse(localStorage.getItem('insyndoc.preferences.v1') ?? '{}'));
  expect(preferences.spellingWords).toContain('zorbulate');
});

test('Grammar Review Loads On Request And Releases Its Worker After Returning Suggestions', async ({ page }) => {
  let liveWorkers = 0;
  let grammarRequests = 0;
  page.on('worker', (worker) => { liveWorkers++; worker.on('close', () => liveWorkers--); });
  page.on('request', (request) => { if (/harper.*\.wasm/.test(request.url())) grammarRequests++; });
  await page.addInitScript(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: true, updateNotifications: false, autoRecovery: false })));
  await page.goto('http://127.0.0.1:1420');
  const editor = page.getByRole('textbox', { name: 'Document content' });
  await expect(editor).toBeVisible();
  await editor.focus();
  await page.keyboard.insertText('I want to me ready.');
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: /^spelling suggestions$/i });
  await expect(popup.getByRole('button', { name: 'be', exact: true })).toBeVisible();
  expect(grammarRequests).toBe(1);
  await expect.poll(() => liveWorkers).toBe(0);
  await popup.getByRole('button', { name: 'be', exact: true }).click();
  await expect(editor).toContainText('I want to be ready.');
  await editor.focus();
  await page.keyboard.press('Control+End');
  await page.keyboard.insertText(' profromace');
  await expect(page.locator('.spelling-mark').first()).toBeAttached();
  expect(grammarRequests).toBe(1);
});

test('Editing Cancels An Unfinished Grammar Review And Resumes Automatic Spelling', async ({ page }) => {
  let grammarArrived: () => void = () => undefined;
  let releaseGrammar: () => void = () => undefined;
  const arrived = new Promise<void>((resolve) => { grammarArrived = resolve; });
  const released = new Promise<void>((resolve) => { releaseGrammar = resolve; });
  await page.route(/harper.*\.wasm/, async (route) => { grammarArrived(); await released; await route.continue().catch(() => undefined); });
  try {
    await page.addInitScript(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: true, updateNotifications: false, autoRecovery: false })));
    await page.goto('http://127.0.0.1:1420');
    const editor = page.getByRole('textbox', { name: 'Document content' });
    await expect(editor).toBeVisible();
    await editor.focus();
    await page.keyboard.insertText('I want to me ready.');
    await page.keyboard.press('Control+Alt+x');
    await arrived;
    await editor.focus();
    await page.keyboard.press('Control+End');
    await page.keyboard.insertText(' profromace');
    releaseGrammar();
    await expect(page.locator('.spelling-mark').first()).toBeAttached();
    await expect(page.getByRole('dialog', { name: /^spelling suggestions$/i })).not.toBeVisible();
    await expect(page.locator('.spelling-error')).toHaveCount(0);
  } finally {
    releaseGrammar();
  }
});

test('A Failed Grammar Review Leaves Automatic Spelling Available', async ({ page }) => {
  await page.route(/harper.*\.wasm/, (route) => route.abort());
  await page.addInitScript(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: true, updateNotifications: false, autoRecovery: false })));
  await page.goto('http://127.0.0.1:1420');
  const editor = page.getByRole('textbox', { name: 'Document content' });
  await expect(editor).toBeVisible();
  await editor.focus();
  await page.keyboard.insertText('I want to me ready.');
  await page.keyboard.press('Control+Alt+x');
  await expect(page.locator('.spelling-error')).toContainText('Grammar Review Is Unavailable');
  await editor.focus();
  await page.keyboard.press('Control+End');
  await page.keyboard.insertText(' profromace');
  await expect(page.locator('.spelling-mark').first()).toBeAttached();
  await page.mouse.move(5, 5);
  await expect(page.getByRole('dialog', { name: /^spelling suggestions$/i })).not.toBeVisible();
});
