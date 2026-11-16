import { expect, test } from '@playwright/test';
import { INTERFACE_STYLES } from '../../src/preferences';

test('Switch Interfaces Without Replacing The Document And Restore The Saved Choice', async ({ page }, testInfo) => {
  await page.addInitScript(() => {
    if (!localStorage.getItem('insyndoc.preferences.v1')) {
      localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: false, updateNotifications: false }));
    }
  });
  await page.goto('http://127.0.0.1:1420');
  const editor = page.getByRole('textbox', { name: 'Document content' });
  await expect(editor).toBeVisible();
  await editor.focus();
  await page.keyboard.insertText('A Quiet Place To Think');
  await page.getByRole('combobox', { name: 'Paragraph style', exact: true }).click();
  await page.getByRole('option', { name: 'Heading 1', exact: true }).click();
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await page.getByRole('combobox', { name: 'Paragraph style', exact: true }).click();
  await page.getByRole('option', { name: 'Normal', exact: true }).click();
  await page.keyboard.insertText('Every Good Document Starts With A Clear Idea. Make Room For The Words That Matter, Then Shape Them Into Something Worth Sharing.');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');
  await page.keyboard.insertText('Project Notes');
  await page.keyboard.press('Enter');
  await page.keyboard.insertText('Gather The Ideas. Find The Thread. Write The Next Page.');
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({ path: testInfo.outputPath('default-studio.png') });

  for (const theme of ['Light', 'Dark']) {
    for (const style of INTERFACE_STYLES) {
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      const settings = page.getByRole('dialog', { name: 'Settings', exact: true });
      await expect(settings.getByRole('group', { name: 'Interface style', exact: true }).getByRole('button')).toHaveCount(4);
      await settings.getByRole('button', { name: theme, exact: true }).click();
      const option = settings.getByRole('button', { name: style.label, exact: true });
      await option.click();
      await expect(option).toHaveAttribute('aria-pressed', 'true');
      await expect(page.locator('html')).toHaveAttribute('data-interface', style.value);
      await settings.getByRole('button', { name: 'Done', exact: true }).click();
      await expect(editor).toContainText('A Quiet Place To Think');
      await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeVisible();
      if (theme === 'Light') {
        await page.getByRole('combobox', { name: 'Paragraph style', exact: true }).click();
        await expect(page.getByRole('option', { name: 'Heading 1', exact: true })).toBeInViewport();
        await page.keyboard.press('Escape');
        await page.getByRole('button', { name: 'Download document', exact: true }).click();
        await expect(page.getByRole('menuitem', { name: 'Markdown (.md)', exact: true })).toBeInViewport();
        await page.keyboard.press('Escape');
        await page.getByRole('button', { name: 'Hide menus', exact: true }).click();
        await page.getByRole('button', { name: 'Exit focus mode' }).click();
      }
      await page.screenshot({ path: testInfo.outputPath(`${style.value}-${theme.toLowerCase()}.png`) });
    }
  }

  await page.reload();
  await expect(editor).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('data-interface', 'studio');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.keyboard.press('Control+,');
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true }).getByRole('button', { name: 'Studio', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await page.getByRole('group', { name: 'Interface style', exact: true }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('interface-settings.png') });
});

test('All Interfaces Keep Settings And Editing Accessible In A Narrow Window', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.addInitScript(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: false, updateNotifications: false })));
  await page.goto('http://127.0.0.1:1420');
  await expect(page.getByRole('textbox', { name: 'Document content' })).toBeVisible();
  for (const style of INTERFACE_STYLES) {
    await page.keyboard.press('Control+,');
    const settings = page.getByRole('dialog', { name: 'Settings', exact: true });
    await settings.getByRole('button', { name: style.label, exact: true }).click();
    await settings.getByRole('button', { name: 'Done', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeInViewport();
    await expect(page.locator('.toolbar-shell')).toBeInViewport();
    await page.screenshot({ path: testInfo.outputPath(`${style.value}-narrow.png`) });
  }
});

test('Rail Menus Support Vertical Keyboard Navigation And Short Windows', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ interfaceStyle: 'rail', checkSpelling: false, updateNotifications: false })));
  await page.goto('http://127.0.0.1:1420');
  await expect(page.getByRole('textbox', { name: 'Document content' })).toBeVisible();
  const menuBar = page.getByRole('menubar');
  await expect(menuBar).toHaveAttribute('aria-orientation', 'vertical');
  await menuBar.getByRole('menuitem', { name: 'File', exact: true }).focus();
  await page.keyboard.press('ArrowDown');
  await expect(menuBar.getByRole('menuitem', { name: 'Edit', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('menu', { name: 'Edit', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await page.keyboard.press('ArrowUp');
  await expect(menuBar.getByRole('menuitem', { name: 'File', exact: true })).toBeFocused();
  await page.setViewportSize({ width: 1280, height: 600 });
  for (const name of ['File', 'Edit', 'View', 'Insert', 'Format', 'Tools', 'Help']) {
    await menuBar.getByRole('menuitem', { name, exact: true }).click();
    await expect(page.getByRole('menu', { name, exact: true })).toBeInViewport({ ratio: 1 });
    await page.keyboard.press('Escape');
  }
  await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeInViewport();
  await page.setViewportSize({ width: 800, height: 600 });
  await expect(menuBar).toHaveAttribute('aria-orientation', 'horizontal');
});
