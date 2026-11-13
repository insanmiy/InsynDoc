import { expect, test, type Locator, type Page } from '@playwright/test';

interface SaveRequest {
  token: string | null;
  saveAs: boolean;
  name?: string;
  activate?: boolean;
}

interface FeatureTestState {
  saves: SaveRequest[];
  counts: string[];
  cancelSaveAs: boolean;
  delaySelectionCount: boolean;
  finishSelectionCount?: () => void;
}

async function openEditor(page: Page): Promise<Locator> {
  await page.addInitScript(() => {
    localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: false, updateNotifications: false }));
    const desktopWindow = window as unknown as {
      isTauri: boolean;
      __featureTest: FeatureTestState;
      __TAURI_INTERNALS__: unknown;
    };
    const state: FeatureTestState = { saves: [], counts: [], cancelSaveAs: false, delaySelectionCount: false };
    desktopWindow.isTauri = true;
    desktopWindow.__featureTest = state;
    desktopWindow.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' } },
      invoke: async (command: string, args: unknown = {}) => {
        if (command === 'open_startup_document') return new Uint8Array([4, 0, 0, 0, 110, 117, 108, 108]).buffer;
        if (command === 'installed_fonts') return [];
        if (command === 'caps_lock_on') return false;
        if (command === 'save_document') {
          const packet = args instanceof Uint8Array ? args : new Uint8Array(args as ArrayBuffer);
          const length = new DataView(packet.buffer, packet.byteOffset, packet.byteLength).getUint32(0, true);
          const metadata: SaveRequest = JSON.parse(new TextDecoder().decode(packet.subarray(4, 4 + length)));
          state.saves.push(metadata);
          if (metadata.activate && state.cancelSaveAs) { state.cancelSaveAs = false; return null; }
          if (metadata.activate) return { token: 'copy-token', name: 'Copy.docx' };
          if (metadata.saveAs) return { token: 'export-token', name: 'Export.docx' };
          return { token: metadata.token ?? 'original-token', name: metadata.token ? metadata.name : 'Original.docx' };
        }
        if (command === 'document_statistics') {
          const { text = '' } = args as { text?: string };
          state.counts.push(text);
          const counts = {
            words: Array.from(new Intl.Segmenter('en', { granularity: 'word' }).segment(text)).filter((entry) => entry.isWordLike).length,
            characters: Array.from(text).length,
            charactersWithoutSpaces: Array.from(text).filter((character) => !/\s/u.test(character)).length
          };
          if (state.delaySelectionCount && text === 'Hello') {
            return new Promise((resolve) => { state.finishSelectionCount = () => resolve(counts); });
          }
          return counts;
        }
        return null;
      }
    };
  });
  await page.goto('http://127.0.0.1:1420');
  const editor = page.getByRole('textbox', { name: 'Document Content' });
  await expect(editor).toBeVisible();
  await editor.focus();
  return editor;
}

async function fileAction(page: Page, name: string): Promise<void> {
  await page.locator('.docs-menu').getByText('File', { exact: true }).click();
  await page.getByRole('menuitem', { name }).click();
}

async function expectStatistic(section: Locator, name: string, value: string): Promise<void> {
  await expect(section.locator('dt').filter({ hasText: new RegExp(`^${name}$`) }).locator('..').locator('dd')).toHaveText(value);
}

test('Save As Changes The Active Document And Export Remains A Copy', async ({ page }) => {
  const editor = await openEditor(page);
  await page.keyboard.insertText('Original Text');
  await page.keyboard.press('Control+s');
  await expect(page).toHaveTitle('Original.docx | InsynDoc');
  await fileAction(page, 'Save As...');
  await expect(page).toHaveTitle('Copy.docx | InsynDoc');
  const saves = await page.evaluate(() => (window as unknown as { __featureTest: FeatureTestState }).__featureTest.saves);
  expect(saves[1]).toMatchObject({ token: 'original-token', saveAs: true, activate: true });
  await editor.focus();
  await page.keyboard.press('Control+End');
  await page.keyboard.insertText(' More Text');
  await page.keyboard.press('Control+s');
  await expect(page).toHaveTitle('Copy.docx | InsynDoc');
  await fileAction(page, 'Export DOCX');
  await expect(page.getByRole('status').filter({ hasText: 'Exported Export.docx' })).toBeVisible();
  await expect(page).toHaveTitle('Copy.docx | InsynDoc');
  const finalSaves = await page.evaluate(() => (window as unknown as { __featureTest: FeatureTestState }).__featureTest.saves);
  expect(finalSaves[2]).toMatchObject({ token: 'copy-token', saveAs: false });
  expect(finalSaves[3]).toMatchObject({ token: 'copy-token', saveAs: true });
  expect(finalSaves[3].activate).toBeUndefined();
});

test('Cancelling Save As Preserves The Original Name And Unsaved Edits', async ({ page }) => {
  const editor = await openEditor(page);
  await page.keyboard.insertText('Original Text');
  await page.keyboard.press('Control+s');
  await expect(page).toHaveTitle('Original.docx | InsynDoc');
  await editor.focus();
  await page.keyboard.insertText(' Pending Edit');
  await page.evaluate(() => { (window as unknown as { __featureTest: FeatureTestState }).__featureTest.cancelSaveAs = true; });
  await fileAction(page, 'Save As...');
  await expect(page).toHaveTitle('* Original.docx | InsynDoc');
  await expect(editor).toContainText('Pending Edit');
});

test('Counts A Selection On Demand And Clears It When The Caret Collapses', async ({ page }) => {
  const editor = await openEditor(page);
  await page.keyboard.insertText('Hello Wonderful World');
  await expect(page.getByRole('button', { name: 'Word Count', exact: true })).toHaveText('3 Words');
  await page.keyboard.press('Control+Home');
  await page.keyboard.down('Shift');
  for (let index = 0; index < 5; index++) await page.keyboard.press('ArrowRight');
  await page.keyboard.up('Shift');
  expect(await page.evaluate(() => (window as unknown as { __featureTest: FeatureTestState }).__featureTest.counts.includes('Hello'))).toBe(false);
  await page.getByRole('button', { name: 'Word Count', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Word Count', exact: true });
  const selection = dialog.getByRole('region', { name: 'Selected Text Statistics' });
  await expectStatistic(selection, 'Words', '1');
  await expectStatistic(selection, 'Characters', '5');
  await expectStatistic(selection, 'Characters Without Spaces', '5');
  await expectStatistic(dialog.getByRole('region', { name: 'Document Statistics' }), 'Words', '3');
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  await editor.focus();
  await page.keyboard.press('Control+End');
  await page.getByRole('button', { name: 'Word Count', exact: true }).click();
  await expect(dialog).toBeVisible();
  await expect(selection).toHaveCount(0);
  await expectStatistic(dialog.getByRole('region', { name: 'Document Statistics' }), 'Words', '3');
});

test('Selection Counts Preserve Unicode Characters And Spaces', async ({ page }) => {
  await openEditor(page);
  await page.keyboard.insertText('Café 🙂 Second');
  await page.keyboard.press('Control+a');
  await page.getByRole('button', { name: 'Word Count', exact: true }).click();
  const selection = page.getByRole('dialog', { name: 'Word Count', exact: true }).getByRole('region', { name: 'Selected Text Statistics' });
  await expectStatistic(selection, 'Words', '2');
  await expectStatistic(selection, 'Characters', '13');
  await expectStatistic(selection, 'Characters Without Spaces', '11');
  await expectStatistic(page.getByRole('region', { name: 'Document Statistics' }), 'Words', '2');
});

test('Ignores A Late Count From A Previously Closed Dialog', async ({ page }) => {
  const editor = await openEditor(page);
  await page.keyboard.insertText('Hello Wonderful World');
  await page.keyboard.press('Control+Home');
  await page.keyboard.down('Shift');
  for (let index = 0; index < 5; index++) await page.keyboard.press('ArrowRight');
  await page.keyboard.up('Shift');
  await page.evaluate(() => { (window as unknown as { __featureTest: FeatureTestState }).__featureTest.delaySelectionCount = true; });
  await page.getByRole('button', { name: 'Word Count', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Word Count', exact: true });
  await expect(dialog.getByRole('status')).toHaveText('Counting Text...');
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  await editor.focus();
  await page.keyboard.press('Control+Home');
  await page.keyboard.down('Shift');
  for (let index = 0; index < 3; index++) await page.keyboard.press('ArrowRight');
  await page.keyboard.up('Shift');
  await page.evaluate(() => { (window as unknown as { __featureTest: FeatureTestState }).__featureTest.delaySelectionCount = false; });
  await page.getByRole('button', { name: 'Word Count', exact: true }).click();
  const selection = dialog.getByRole('region', { name: 'Selected Text Statistics' });
  await expectStatistic(selection, 'Characters', '3');
  await page.evaluate(async () => {
    (window as unknown as { __featureTest: FeatureTestState }).__featureTest.finishSelectionCount?.();
    await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
  });
  await expectStatistic(selection, 'Characters', '3');
});
