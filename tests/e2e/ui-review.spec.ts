import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: false, updateNotifications: false })));
  await page.goto('http://127.0.0.1:1420');
  await expect(page.getByRole('textbox', { name: 'Document content' })).toBeVisible();
});

test('Download Selection Returns Keyboard Focus To Its Trigger', async ({ page }) => {
  const trigger = page.getByRole('button', { name: 'Download document' });
  await trigger.click();
  await page.getByRole('menuitem', { name: 'Markdown (.md)', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('menu', { name: 'Download formats' })).not.toBeVisible();
  await expect(trigger).toBeFocused();
});

test('Arrow Up Opens The Last Download Format And Settings Dismisses The Menu', async ({ page }) => {
  await page.getByRole('button', { name: 'Download document' }).focus();
  await page.keyboard.press('ArrowUp');
  await expect(page.getByRole('menuitem', { name: 'Markdown (.md)', exact: true })).toBeFocused();
  await page.keyboard.press('Control+,');
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeVisible();
  await expect(page.getByRole('menu', { name: 'Download formats' })).not.toBeVisible();
});

test('Closing A Conditionally Mounted Dialog Returns Focus To Its Menu', async ({ page }) => {
  await page.locator('.docs-menu').getByText('File', { exact: true }).click();
  await page.getByRole('menuitem').filter({ hasText: 'Templates' }).click();
  await expect(page.getByRole('dialog', { name: 'Document templates' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('.docs-menu').getByText('File', { exact: true })).toBeFocused();
});

test('Tab Leaves The Download Menu In Both Directions', async ({ page }) => {
  const trigger = page.getByRole('button', { name: 'Download document' });
  await trigger.click();
  await page.keyboard.press('Shift+Tab');
  await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeFocused();
  await expect(page.getByRole('menu', { name: 'Download formats' })).not.toBeVisible();
  await trigger.click();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('button', { name: 'Menus', exact: true })).toBeFocused();
  await expect(page.getByRole('menu', { name: 'Download formats' })).not.toBeVisible();
});
