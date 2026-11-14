import { expect, test } from '@playwright/test';

test('Updates Page Counts After Bulk Paste, Undo, And Redo', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: false, updateNotifications: false }));
    const desktopWindow = window as unknown as {
      isTauri: boolean;
      __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: string } };
        invoke: (command: string) => Promise<unknown>;
      };
    };
    desktopWindow.isTauri = true;
    desktopWindow.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' } },
      invoke: async (command) => {
        if (command === 'open_startup_document') return new Uint8Array([4, 0, 0, 0, 110, 117, 108, 108]).buffer;
        if (command === 'installed_fonts') return [];
        if (command === 'caps_lock_on') return false;
        if (command === 'document_statistics') return { words: 0, characters: 0, charactersWithoutSpaces: 0 };
        return null;
      }
    };
  });
  await page.goto('http://127.0.0.1:1420');
  const editor = page.getByRole('textbox', { name: 'Document content' });
  await expect(editor).toBeVisible();
  await editor.focus();
  const text = Array.from({ length: 200 }, (_, index) => `Paragraph ${index + 1}. ${'Document Text For Page Count Verification. '.repeat(10)}`).join('\n');
  await editor.evaluate((element, content) => {
    const clipboardData = new DataTransfer();
    clipboardData.setData('text/plain', content);
    element.dispatchEvent(new ClipboardEvent('paste', { clipboardData, bubbles: true, cancelable: true }));
  }, text);
  await expect(editor).toContainText('Paragraph 200.');
  const pageCount = page.locator('.document-status button[aria-label="Go to page"]');
  await expect.poll(async () => Number((await pageCount.innerText()).match(/\d+/)?.[0])).toBeGreaterThan(1);
  const expectedPages = await page.locator('.docx-page').count();
  await expect(pageCount).toHaveText(`${expectedPages} pages`);
  await pageCount.click();
  const dialog = page.getByRole('dialog', { name: 'Go to page', exact: true });
  await expect(dialog.locator('#page-number')).toHaveAttribute('max', String(expectedPages));
  await dialog.locator('#page-number').fill('1');
  await dialog.getByRole('button', { name: 'Go to page', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await editor.focus();
  await page.keyboard.press('Control+z');
  await expect(pageCount).toHaveText('1 page');
  await expect(editor).not.toContainText('Paragraph 1.');
  await page.keyboard.press('Control+y');
  await expect(pageCount).toHaveText(`${expectedPages} pages`);
  await expect(editor).toContainText('Paragraph 200.');
});
