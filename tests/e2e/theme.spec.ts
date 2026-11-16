import { expect, test } from '@playwright/test';

test('System theme applies immediately after a manual theme and follows later system changes', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ theme: 'light', checkSpelling: false, updateNotifications: false }));
    type Theme = 'light' | 'dark';
    const state = {
      systemTheme: 'dark' as Theme,
      nativeTheme: null as Theme | null,
      setSystemTheme(theme: Theme) {
        state.systemTheme = theme;
        media.dispatchEvent(new Event('change'));
      }
    };
    const media = Object.defineProperty(new EventTarget(), 'matches', { get: () => (state.nativeTheme ?? state.systemTheme) === 'dark' });
    const originalMatchMedia = window.matchMedia.bind(window);
    window.matchMedia = (query: string) => query === '(prefers-color-scheme: dark)' ? media as MediaQueryList : originalMatchMedia(query);
    const desktopWindow = window as unknown as {
      isTauri: boolean;
      __themeTest: typeof state;
      __TAURI_INTERNALS__: unknown;
    };
    desktopWindow.isTauri = true;
    desktopWindow.__themeTest = state;
    desktopWindow.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' } },
      invoke: async (command: string, args: unknown = {}) => {
        if (command === 'plugin:window|set_theme') {
          state.nativeTheme = (args as { value: Theme | null }).value;
          media.dispatchEvent(new Event('change'));
        }
        if (command === 'open_startup_document') return new Uint8Array([4, 0, 0, 0, 110, 117, 108, 108]).buffer;
        if (command === 'installed_fonts' || command === 'recent_documents') return [];
        if (command === 'caps_lock_on') return false;
        if (command === 'document_statistics') return { words: 0, characters: 0, charactersWithoutSpaces: 0 };
        return null;
      }
    };
  });
  await page.goto('http://127.0.0.1:1420');
  await expect(page.getByRole('textbox', { name: 'Document content' })).toBeVisible();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const settings = page.getByRole('dialog', { name: 'Settings', exact: true });
  await settings.getByRole('button', { name: 'System', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect.poll(() => page.evaluate(() => (window as unknown as { __themeTest: { nativeTheme: string | null } }).__themeTest.nativeTheme)).toBeNull();
  await page.evaluate(() => (window as unknown as { __themeTest: { setSystemTheme: (theme: string) => void } }).__themeTest.setSystemTheme('light'));
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await settings.getByRole('button', { name: 'Dark', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await settings.getByRole('button', { name: 'System', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
});

test('Browser system theme follows appearance changes without restarting', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ theme: 'light', checkSpelling: false, updateNotifications: false })));
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto('http://127.0.0.1:1420');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const settings = page.getByRole('dialog', { name: 'Settings', exact: true });
  await settings.getByRole('button', { name: 'System', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await settings.getByRole('button', { name: 'Dark', exact: true }).click();
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
});
