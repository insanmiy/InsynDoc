import { expect, test, type Page } from '@playwright/test';
import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import { writeFile } from 'node:fs/promises';
import type { RecentDocument } from '../../src/bridge';
import additionalFontFamilies from '../../src/document-font-families.json' with { type: 'json' };

interface NativeState {
  downloads: Array<{ format: string; name: string; bytes: number[]; text?: string }>;
  cancelDownload: boolean;
  failDownload: boolean;
  openedLinks: Array<{ url: string; browserPath: string | null }>;
  browserChoice: string | null;
  failOpenLink: boolean;
  saveRequests: Array<{ token: unknown; saveAs: unknown }>;
  saved: number[] | null;
  openBytes: number[] | null;
  cancelSave: boolean;
  failSave: boolean;
  closed: boolean;
  requestClose: (() => void) | null;
  recent: RecentDocument[];
  exported: string | null;
}

declare global {
  interface Window {
    __nativeTest: NativeState;
  }
}

function fixture(): number[] {
  return Array.from(zipSync({
    '[Content_Types].xml': strToU8('<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>'),
    '_rels/.rels': strToU8('<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>'),
    'word/document.xml': strToU8('<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Original Mixed CASE Content</w:t></w:r></w:p><w:p><w:r><w:br w:type="page"/></w:r></w:p><w:p><w:r><w:t>Second Page</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>')
  }));
}

function importedHeadingFixture(formatted = false): number[] {
  const parts = unzipSync(new Uint8Array(fixture()));
  const paragraph = formatted
    ? '<w:p><w:pPr><w:jc w:val="center"/><w:spacing w:before="180"/><w:rPr><w:i/><w:color w:val="AA00AA"/></w:rPr></w:pPr><w:r><w:rPr><w:b/><w:color w:val="FF0000"/><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri"/></w:rPr><w:t xml:space="preserve">Bold Red </w:t></w:r><w:r><w:rPr><w:i/><w:color w:val="008000"/><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr><w:t>Italic Green</w:t></w:r></w:p>'
    : '<w:p/>';
  parts['word/document.xml'] = strToU8(strFromU8(parts['word/document.xml']).replace('<w:p><w:r><w:t>Original Mixed CASE Content</w:t></w:r></w:p>', paragraph));
  parts['[Content_Types].xml'] = strToU8(strFromU8(parts['[Content_Types].xml']).replace('</Types>', '<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>'));
  parts['word/_rels/document.xml.rels'] = strToU8('<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>');
  parts['word/styles.xml'] = strToU8('<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val="22"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:rPr><w:sz w:val="22"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="Heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:sz w:val="40"/></w:rPr></w:style></w:styles>');
  return Array.from(zipSync(parts));
}

function coloredFixture(): number[] {
  const parts = unzipSync(new Uint8Array(fixture()));
  const xml = strFromU8(parts['word/document.xml']).replace('<w:r><w:t>Original', '<w:r><w:rPr><w:color w:val="C62026"/></w:rPr><w:t>Original');
  parts['word/document.xml'] = strToU8(xml);
  return Array.from(zipSync(parts));
}

function hyperlinkFixture(label = 'Hello World!', target = 'https://example.com/'): number[] {
  const parts = unzipSync(new Uint8Array(fixture()));
  parts['word/document.xml'] = strToU8(strFromU8(parts['word/document.xml']).replace('<w:r><w:t>Original Mixed CASE Content</w:t></w:r>', `<w:r><w:t xml:space="preserve">Before </w:t></w:r><w:hyperlink r:id="rLink" w:tooltip="Original Tooltip" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:r><w:rPr><w:b/></w:rPr><w:t>${label}</w:t></w:r></w:hyperlink><w:r><w:t xml:space="preserve"> ${label}</w:t></w:r>`));
  parts['word/_rels/document.xml.rels'] = strToU8(`<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="${target}" TargetMode="External"/></Relationships>`);
  return Array.from(zipSync(parts));
}

async function fileAction(page: Page, action: 'New' | 'Open'): Promise<void> {
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: new RegExp(`^${action}\\s+Ctrl\\+`) }).click();
}

async function switchView(page: Page, view: 'Continuous' | 'Separate Pages'): Promise<void> {
  await page.getByRole('menuitem', { name: 'View', exact: true }).click();
  await page.getByRole('menuitemcheckbox', { name: view, exact: true }).click();
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const native: NativeState = { downloads: [], cancelDownload: false, failDownload: false, openedLinks: [], browserChoice: null, failOpenLink: false, saveRequests: [], saved: null, openBytes: null, cancelSave: false, failSave: false, closed: false, requestClose: null, recent: [], exported: null };
    const callbacks = new Map<number, (event: unknown) => void>();
    let nextCallback = 1;
    const documentResponse = (token: string, name: string, bytes = native.openBytes): ArrayBuffer => {
      const metadata = new TextEncoder().encode(JSON.stringify(bytes ? { token, name } : null));
      const packet = new Uint8Array(4 + metadata.length + (bytes?.length ?? 0));
      new DataView(packet.buffer).setUint32(0, metadata.length, true);
      packet.set(metadata, 4);
      packet.set(bytes ?? [], 4 + metadata.length);
      return packet.buffer;
    };
    Object.assign(window, {
      isTauri: true,
      __nativeTest: native,
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => undefined },
      __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: 'main' } },
        transformCallback(callback: (event: unknown) => void) { const identifier = nextCallback++; callbacks.set(identifier, callback); return identifier; },
        unregisterCallback(identifier: number) { callbacks.delete(identifier); },
        async invoke(command: string, request?: Record<string, unknown> | Uint8Array) {
          const packet = request instanceof Uint8Array ? request : undefined;
          const metadataLength = packet ? new DataView(packet.buffer, packet.byteOffset, packet.byteLength).getUint32(0, true) : 0;
          const args: Record<string, unknown> | undefined = packet
            ? JSON.parse(new TextDecoder().decode(packet.subarray(4, 4 + metadataLength))) as Record<string, unknown>
            : request as Record<string, unknown> | undefined;
          if (command === 'choose_link_browser') return native.browserChoice;
          if (command === 'open_external_link') {
            if (native.failOpenLink) throw new Error('Could Not Open The Selected Browser.');
            native.openedLinks.push({ url: String(args?.url), browserPath: typeof args?.browserPath === 'string' ? args.browserPath : null });
            return;
          }
          if (command === 'recent_documents') return native.recent;
          if (command === 'export_plain_text') { native.exported = String(args?.text); return 'Export.txt'; }
          if (command === 'clear_recent_documents') { native.recent = []; return; }
          if (command === 'open_recent_document') return documentResponse(String(args?.token), 'Recent.docx');
          if (command === 'document_statistics') {
            const text = String(args?.text ?? '');
            return { words: Array.from(new Intl.Segmenter(undefined, { granularity: 'word' }).segment(text)).filter((word) => word.isWordLike).length, characters: Array.from(text).length, charactersWithoutSpaces: Array.from(text).filter((character) => !/\s/u.test(character)).length };
          }
          if (command === 'open_document') return documentResponse('opened', 'Example.docx');
          if (command === 'open_startup_document') {
            const startup = JSON.parse(sessionStorage.getItem('insyndoc.test.startup') ?? 'null') as { name: string; bytes: number[]; error?: string } | null;
            sessionStorage.removeItem('insyndoc.test.startup');
            if (startup?.error) throw new Error(startup.error);
            return documentResponse('startup', startup?.name ?? 'Untitled.docx', startup?.bytes ?? null);
          }
          if (command === 'rename_document') return { token: 'renamed', name: args?.name };
          if (command === 'save_document') {
            native.saveRequests.push({ token: args?.token, saveAs: args?.saveAs });
            if (native.failSave) throw new Error('Could Not Replace The Document.');
            if (native.cancelSave) return null;
            native.saved = packet ? Array.from(packet.subarray(4 + metadataLength)) : null;
            if (args?.saveAs) return { token: 'exported-copy', name: 'Exported.docx' };
            return { token: 'saved', name: 'Example.docx' };
          }
          if (command === 'download_document' || command === 'download_pdf') {
            if (native.failDownload) throw new Error('The Download Could Not Be Saved.');
            if (native.cancelDownload) return null;
            const format = command === 'download_pdf' ? 'pdf' : String(args?.format);
            native.downloads.push({ format, name: String(args?.name), bytes: packet ? Array.from(packet.subarray(4 + metadataLength)) : [], text: document.getElementById('pdf-export-root')?.textContent ?? undefined });
            return `Untitled.${format === 'html' ? 'zip' : format}`;
          }
          if (command === 'plugin:window|destroy') native.closed = true;
          if (command === 'plugin:event|listen') {
            if (args?.event === 'tauri://close-requested') native.requestClose = () => callbacks.get(Number(args.handler))?.({ event: 'tauri://close-requested', id: 1, payload: null });
            return 1;
          }
          return null;
        }
      }
    });
  });
  await page.goto('http://127.0.0.1:1420');
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
  await expect(page.locator('.docx-page').first()).toBeVisible();
  await expect(page.locator('.document-save-indicator')).toHaveText('New Document');
});

for (const marker of ['*', '-', '+', '1.', '1)']) {
  test(`Automatically Formats ${marker} Space As A List`, async ({ page }) => {
    await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
    await page.keyboard.type(`${marker} First Item`);
    await page.keyboard.press('Enter');
    await page.keyboard.type('Second Item');
    await page.keyboard.press('Control+s');
    await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
    const saved = await page.evaluate(() => window.__nativeTest.saved);
    const parts = unzipSync(new Uint8Array(saved!));
    const xml = strFromU8(parts['word/document.xml']);
    expect(xml.match(/<w:numPr>/g)).toHaveLength(2);
    expect(xml).not.toContain(`${marker} `);
    expect(strFromU8(parts['word/numbering.xml'])).toContain(`w:val="${marker.startsWith('1') ? 'decimal' : 'bullet'}"`);
    await page.screenshot({ path: `artifacts/ui/autoformat-${marker === '*' ? 'bullet' : marker === '1.' ? 'numbered' : 'list'}-proof.png` });
  });
}

for (const marker of ['*', '-', '+', '1.', '1)', '#', '##', '###', '####', '#####', '######']) {
  test(`Backspace Restores The Literal ${marker} Marker And Space`, async ({ page }) => {
    const fontSize = page.getByRole('combobox', { name: 'Font Size', exact: true });
    const normalSize = await fontSize.inputValue();
    const content = page.locator('.docx-page-content').first();
    await content.click({ position: { x: 30, y: 12 } });
    await page.keyboard.type(`${marker} `);
    if (marker.startsWith('#')) {
      await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText(`Heading ${marker.length}`);
    }
    else await expect(page.locator('.docx-list-marker')).toHaveCount(1);
    await page.keyboard.press('Backspace');
    expect(await content.textContent()).toBe(`${marker} `);
    await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText('Normal');
    await expect(fontSize).toHaveValue(normalSize);
    await expect(page.locator('.docx-list-marker')).toHaveCount(0);
    await page.keyboard.type('Literal Text');
    await expect(content).toContainText(`${marker} Literal Text`);
    await page.keyboard.press('Control+s');
    await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
    const saved = await page.evaluate(() => window.__nativeTest.saved);
    const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
    expect(xml).not.toContain('<w:numPr>');
    expect(xml).not.toContain('w:pStyle w:val="Heading');
    if (marker === '##') await page.screenshot({ path: 'artifacts/ui/backspace-shortcut-proof.png' });
  });
}

test('Backspace Deletes Normally After Typing Or Moving Away From A Shortcut', async ({ page }) => {
  const content = page.locator('.docx-page-content').first();
  await content.click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('## Example');
  await page.keyboard.press('Backspace');
  await expect(content).toContainText('Exampl');
  await page.keyboard.press('Control+z');
  await expect(content).toContainText('Example');
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Backspace');
  await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
  await page.getByRole('option', { name: 'Normal', exact: true }).click();
  await page.keyboard.type('* ');
  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('Backspace');
  expect(await content.textContent()).not.toContain('* ');
});

test('A Second Backspace Deletes The Restored Space Normally', async ({ page }) => {
  const content = page.locator('.docx-page-content').first();
  await content.click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('## ');
  await page.keyboard.press('Backspace');
  expect(await content.textContent()).toBe('## ');
  await page.keyboard.press('Backspace');
  expect(await content.textContent()).toBe('##');
});

test('Undoes Automatic Formatting Back To The Literal Marker And Space', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('* ');
  await page.keyboard.press('Control+z');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  let saved = await page.evaluate(() => window.__nativeTest.saved);
  let xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).not.toContain('<w:numPr>');
  expect(await page.locator('.docx-page-content').first().textContent()).toContain('* ');
  await page.keyboard.type('Literal Text');
  await page.keyboard.press('Control+s');
  saved = await page.evaluate(() => window.__nativeTest.saved);
  xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).not.toContain('<w:numPr>');
  expect(await page.locator('.docx-page-content').first().textContent()).toContain('* Literal Text');
});

test('Preserves Existing Paragraph And Mixed Run Formatting During Heading Conversion', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, importedHeadingFixture(true));
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await expect(page.locator('.document-name-button')).toContainText('Example.docx');
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.press('Control+Home');
  await page.keyboard.type('## ');
  await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText('Heading 2');
  const saveXml = async (): Promise<string> => {
    await page.keyboard.press('Control+s');
    await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
    const bytes = await page.evaluate(() => window.__nativeTest.saved);
    return strFromU8(unzipSync(new Uint8Array(bytes!))['word/document.xml']);
  };
  const checkFormatting = (xml: string): void => {
    expect(xml).toContain('<w:jc w:val="center"');
    expect(xml).toContain('w:before="180"');
    const runs = xml.match(/<w:r[ >][\s\S]*?<\/w:r>/g) ?? [];
    const red = runs.find((run) => run.includes('Bold Red'))!;
    const green = runs.find((run) => run.includes('Italic Green'))!;
    expect(red).toMatch(/<w:b(?:\s|\/|>)/);
    expect(red).toContain('FF0000');
    expect(red).toContain('Calibri');
    expect(green).toMatch(/<w:i(?:\s|\/|>)/);
    expect(green).not.toMatch(/<w:b(?:\s|\/|>)/);
    expect(green).toContain('008000');
    expect(green).toContain('Arial');
    expect(xml).toContain('AA00AA');
  };
  const converted = await saveXml();
  checkFormatting(converted);
  expect(converted).toContain('w:pStyle w:val="Heading2"');
  await page.screenshot({ path: 'artifacts/ui/heading-preserved-formatting-proof.png' });
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').first()).toContainText('## Bold Red');
  checkFormatting(await saveXml());
  await page.keyboard.press('Control+Shift+z');
  const redone = await saveXml();
  checkFormatting(redone);
  expect(redone).toContain('w:pStyle w:val="Heading2"');
});

for (const method of ['Shortcut', 'Style Picker']) {
  test(`Preserves Imported Heading Size Using ${method}`, async ({ page }) => {
    await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, importedHeadingFixture());
    await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
    await expect(page.locator('.document-name-button')).toContainText('Example.docx');
    await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
    await page.keyboard.press('Control+Home');
    if (method === 'Shortcut') await page.keyboard.type('## Imported Heading');
    else {
      await page.keyboard.type('Imported Heading');
      await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
      await page.getByRole('option', { name: 'Heading 2', exact: true }).click();
    }
    await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue('20');
    await page.keyboard.press('Control+s');
    await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
    const bytes = await page.evaluate(() => window.__nativeTest.saved);
    const parts = unzipSync(new Uint8Array(bytes!));
    const xml = strFromU8(parts['word/document.xml']);
    expect(xml).toContain('w:pStyle w:val="Heading2"');
    expect(xml).not.toContain('w:sz w:val="44"');
    expect(strFromU8(parts['word/styles.xml'])).toContain('w:sz w:val="40"');
    await page.screenshot({ path: 'artifacts/ui/heading-imported-size-proof.png' });
  });
}

test('Turns Heading Markers Into Document Heading Styles', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('## Automatic Heading');
  await expect(page.getByRole('button', { name: 'Automatic Heading', exact: true })).toBeVisible();
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue('22');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('w:pStyle w:val="Heading2"');
  expect(xml).not.toContain('## ');
  await page.screenshot({ path: 'artifacts/ui/autoformat-heading-proof.png' });
});

for (const method of ['Shortcut', 'Style Picker'] as const) {
  for (const level of [1, 2, 3]) {
    test(`Returns To Body Font Size After Heading ${level} From ${method}`, async ({ page }) => {
      const fontSize = page.getByRole('combobox', { name: 'Font Size', exact: true });
      const normalSize = await fontSize.inputValue();
      await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
      if (method === 'Shortcut') await page.keyboard.type(`${'#'.repeat(level)} Heading Title`);
      else {
        await page.keyboard.type('Heading Title');
        await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
        await page.getByRole('option', { name: `Heading ${level}`, exact: true }).click();
      }
      const headingSize = await fontSize.inputValue();
      await page.keyboard.press('End');
      await page.keyboard.press('Enter');
      await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText('Normal');
      await expect(fontSize).toHaveValue(normalSize);
      await page.keyboard.press('Control+z');
      await expect(fontSize).toHaveValue(headingSize);
      await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText(`Heading ${level}`);
      await page.keyboard.press('Control+Shift+z');
      await expect(fontSize).toHaveValue(normalSize);
      await page.keyboard.type('This Is Normal Body Text');
      await expect(page.getByRole('navigation', { name: 'Headings', exact: true }).getByRole('button')).toHaveCount(1);
      await page.keyboard.press('Control+s');
      await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
      const saved = await page.evaluate(() => window.__nativeTest.saved);
      const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
      const paragraphs = xml.match(/<w:p[ >][\s\S]*?<\/w:p>/g) ?? [];
      const body = paragraphs.find((paragraph) => paragraph.includes('This Is Normal Body Text'))!;
      expect(body).not.toContain(`w:sz w:val="${Number(headingSize) * 2}"`);
      await page.screenshot({ path: 'artifacts/ui/heading-body-size-proof.png' });
    });
  }
}

test('Keeps Heading Size For Shift Enter And Splitting Inside A Heading', async ({ page }) => {
  const fontSize = page.getByRole('combobox', { name: 'Font Size', exact: true });
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('## Heading Title');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('Still A Heading');
  await expect(fontSize).toHaveValue('22');
  await page.keyboard.press('Home');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('Enter');
  await expect(fontSize).toHaveValue('22');
  await expect(page.getByRole('combobox', { name: 'Paragraph Style', exact: true })).toHaveText('Heading 2');
});

test('Redoes Automatic List Formatting Without Reintroducing The Marker', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('1. ');
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').first()).toContainText('1.');
  await page.keyboard.press('Control+Shift+z');
  await page.keyboard.type('Restored List');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('<w:numPr>');
  expect(xml).not.toContain('1. ');
  await expect(page.locator('.docx-page-content').first()).toContainText('Restored List');
});

test('Restores Custom Font Sizing When Backspace Cancels A Heading Shortcut', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.getByRole('combobox', { name: 'Font Size', exact: true }).fill('18');
  await page.getByRole('combobox', { name: 'Font Size', exact: true }).press('Enter');
  await page.keyboard.type('## ');
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue('22');
  await page.keyboard.press('Backspace');
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue('18');
  expect(await page.locator('.docx-page-content').first().textContent()).toContain('## ');
});

test('Keeps Automatic Formatting Available After Opening Another Document', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, fixture());
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await expect(page.locator('.document-name-button')).toContainText('Example.docx');
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Backspace');
  await page.keyboard.type('* Imported List');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  expect(strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml'])).toContain('<w:numPr>');
});

for (const marker of ['1.', '*']) {
  for (const exitKey of ['Enter', 'Backspace']) {
    test(`Restores The Body Margin After Leaving ${marker} List With ${exitKey}`, async ({ page }) => {
      await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
      await page.keyboard.type('Normal Margin Reference');
      const reference = await page.getByText('Normal Margin Reference', { exact: true }).boundingBox();
      await page.keyboard.press('Enter');
      await page.keyboard.type(`${marker} First Item`);
      await page.keyboard.press('Enter');
      await page.keyboard.type('Second Item');
      await page.keyboard.press('Enter');
      await expect(page.locator('.docx-list-marker')).toHaveCount(3);
      await page.keyboard.press(exitKey);
      await expect(page.locator('.docx-list-marker')).toHaveCount(2);
      await page.keyboard.press('Control+z');
      await expect(page.locator('.docx-list-marker')).toHaveCount(3);
      await page.keyboard.press('Control+Shift+z');
      await expect(page.locator('.docx-list-marker')).toHaveCount(2);
      await page.keyboard.type('Back At The Normal Margin');
      const body = await page.getByText('Back At The Normal Margin', { exact: true }).boundingBox();
      expect(body!.x).toBeCloseTo(reference!.x, 0);
      await page.keyboard.press('Enter');
      await page.keyboard.type('Still At The Normal Margin');
      const next = await page.getByText('Still At The Normal Margin', { exact: true }).boundingBox();
      expect(next!.x).toBeCloseTo(reference!.x, 0);
      await page.keyboard.press('Control+s');
      await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
      const saved = await page.evaluate(() => window.__nativeTest.saved);
      const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
      expect(xml.match(/<w:numPr>/g)).toHaveLength(2);
      await page.screenshot({ path: 'artifacts/ui/list-exit-margin-proof.png' });
    });
  }
}

test('Backspace Outdents An Empty Nested List Before Leaving The List', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Body Margin');
  const reference = await page.getByText('Body Margin', { exact: true }).boundingBox();
  await page.keyboard.press('Enter');
  await page.keyboard.type('1. Parent Item');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Tab');
  await page.keyboard.type('Nested Item');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Backspace');
  await expect(page.locator('.docx-list-marker')).toHaveCount(3);
  await page.keyboard.press('Backspace');
  await expect(page.locator('.docx-list-marker')).toHaveCount(2);
  await page.keyboard.type('Normal After Nested List');
  const body = await page.getByText('Normal After Nested List', { exact: true }).boundingBox();
  expect(body!.x).toBeCloseTo(reference!.x, 0);
});

test('Keeps Existing List Markers Literal And Exits A List On An Empty Enter', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('1. * Literal Marker');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Second Item');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Plain Paragraph');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml.match(/<w:numPr>/g)).toHaveLength(2);
  await expect(page.locator('.docx-page-content').first()).toContainText('* Literal Marker');
  await expect(page.locator('.docx-page-content').first()).toContainText('Plain Paragraph');
});

test('Leaves Inline Markers And Pasted Markers As Literal Text', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Price * Item');
  await page.keyboard.press('Enter');
  await page.keyboard.insertText('* Pasted Text');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).not.toContain('<w:numPr>');
  await expect(page.locator('.docx-page-content').first()).toContainText('Price * Item');
  await expect(page.locator('.docx-page-content').first()).toContainText('* Pasted Text');
});

test('Downloads All Eight Formats From The Header Menu', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Download This Document');
  const formats = [
    ['docx', 'Microsoft Word (.docx)'], ['pdf', 'PDF Document (.pdf)'], ['odt', 'OpenDocument Format (.odt)'],
    ['txt', 'Plain Text (.txt)'], ['rtf', 'Rich Text Format (.rtf)'], ['html', 'Web Page (.html, Zipped)'],
    ['epub', 'EPUB Publication (.epub)'], ['md', 'Markdown (.md)']
  ];
  for (const [format, label] of formats) {
    await page.getByRole('button', { name: 'Download Document', exact: true }).click();
    const menu = page.getByRole('menu', { name: 'Download Formats', exact: true });
    await expect(menu.getByRole('menuitem')).toHaveCount(8);
    if (format === 'docx') await page.screenshot({ path: 'artifacts/ui/download-formats-menu.png' });
    await menu.getByRole('menuitem', { name: label, exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.__nativeTest.downloads.at(-1)?.format)).toBe(format);
    await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
    await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
    await expect(page.locator('#pdf-export-root')).toHaveCount(0);
  }
  const downloads = await page.evaluate(() => window.__nativeTest.downloads);
  for (const download of downloads.filter((entry) => entry.format !== 'pdf')) expect(download.bytes.length).toBeGreaterThan(0);
  expect(downloads.find((entry) => entry.format === 'pdf')?.text).toContain('Download This Document');
  expect(strFromU8(new Uint8Array(downloads.find((entry) => entry.format === 'txt')!.bytes))).toContain('Download This Document');
  expect(await page.evaluate(() => window.__nativeTest.saveRequests)).toEqual([]);
});

test('Keeps The Document Editable After A Cancelled Or Failed Download', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Keep My Changes');
  await page.evaluate(() => { window.__nativeTest.cancelDownload = true; });
  await page.getByRole('button', { name: 'Download Document', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Microsoft Word (.docx)', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
  await expect(page.locator('.workspace-notice')).toHaveCount(0);
  await page.evaluate(() => { window.__nativeTest.cancelDownload = false; window.__nativeTest.failDownload = true; });
  await page.getByRole('button', { name: 'Download Document', exact: true }).click();
  await page.getByRole('menuitem', { name: 'PDF Document (.pdf)', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('The Download Could Not Be Saved.');
  await expect(page.locator('#pdf-export-root')).toHaveCount(0);
  await expect(page.locator('.docx-page-content').first()).toContainText('Keep My Changes');
  await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
});

test('Navigates Download Formats With The Keyboard And Dismisses With Escape', async ({ page }) => {
  await page.getByRole('button', { name: 'Download Document', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('menuitem', { name: 'Microsoft Word (.docx)', exact: true })).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitem', { name: 'PDF Document (.pdf)', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('menu', { name: 'Download Formats', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeFocused();
});

test('Opens A Downloaded DOCX Passed By Windows At Startup', async ({ page }) => {
  await page.evaluate((bytes) => sessionStorage.setItem('insyndoc.test.startup', JSON.stringify({ name: 'Downloaded Report.docx', bytes })), fixture());
  await page.reload();
  await expect(page.locator('.document-name-button')).toContainText('Downloaded Report.docx');
  await expect(page.locator('.docx-page-content').first()).toContainText('Original Mixed CASE Content');
  await page.screenshot({ path: 'artifacts/ui/docx-startup-open.png' });
});

test('Closes Without A Confirmation After All Text Is Deleted', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Temporary Text');
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Backspace');
  await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
  await page.evaluate(() => window.__nativeTest.requestClose?.());
  await expect.poll(() => page.evaluate(() => window.__nativeTest.closed)).toBe(true);
  await expect(page.getByRole('dialog', { name: 'Save Your Changes?', exact: true })).toHaveCount(0);
  expect(await page.evaluate(() => window.__nativeTest.saveRequests)).toEqual([]);
});

test('Offers Profanity As A Normal Spelling Correction', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('fuckk');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions' });
  await expect(popup.getByRole('button', { name: 'fuck', exact: true })).toBeVisible();
  await page.screenshot({ path: 'artifacts/ui/unrestricted-spelling-suggestions.png' });
  await popup.getByRole('button', { name: 'fuck', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toHaveText('fuck');
});

test('Saves With Ctrl S And Exports A Separate Copy With Ctrl Shift S', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Working Document');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  await page.keyboard.type(' With Changes');
  await page.keyboard.press('Control+Shift+s');
  await expect(page.getByRole('status')).toContainText('Exported Exported.docx');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  expect(await page.evaluate(() => window.__nativeTest.saveRequests)).toEqual([
    { token: null, saveAs: false }, { token: 'saved', saveAs: true }, { token: 'saved', saveAs: false }
  ]);
});

test('Imports A DOCX From The Visible Header Button', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, fixture());
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  await expect(page.locator('.docx-page-content').getByText('Original Mixed CASE Content', { exact: true })).toBeVisible();
});

test('Renames Documents From The Title And Cancels Title Edits With Escape', async ({ page }) => {
  await page.getByRole('button', { name: 'Untitled.docx', exact: true }).click();
  await page.getByRole('textbox', { name: 'Document Name', exact: true }).fill('My Report');
  await page.getByRole('textbox', { name: 'Document Name', exact: true }).press('Enter');
  await expect(page.getByRole('heading', { name: 'My Report.docx', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'My Report.docx', exact: true }).click();
  await page.getByRole('textbox', { name: 'Document Name', exact: true }).fill('Cancelled');
  await page.getByRole('textbox', { name: 'Document Name', exact: true }).press('Escape');
  await expect(page.getByRole('heading', { name: 'My Report.docx', exact: true })).toBeVisible();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  await page.getByRole('button', { name: 'Example.docx', exact: true }).click();
  await page.getByRole('textbox', { name: 'Document Name', exact: true }).fill('Final Report');
  await page.getByRole('textbox', { name: 'Document Name', exact: true }).press('Tab');
  await expect(page.getByRole('heading', { name: 'Final Report.docx', exact: true })).toBeVisible();
});

test('Applies Distinct Sizes For Every Heading Even After Direct Font Sizing', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Heading Text');
  await page.getByRole('combobox', { name: 'Font Size', exact: true }).fill('18');
  await page.getByRole('combobox', { name: 'Font Size', exact: true }).press('Enter');
  const sizes = [26, 22, 18, 16, 14, 13, 12, 10, 9];
  for (let index = 0; index < sizes.length; index++) {
    await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
    await page.getByRole('option', { name: `Heading ${index + 1}`, exact: true }).click();
    await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveValue(String(sizes[index]));
  }
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Heading9');
  expect(xml).toContain('w:val="18"');
});

test('Keeps Real Multiline Pasting As Separate Paragraphs', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Start ');
  await page.evaluate(() => {
    const clipboard = new DataTransfer();
    clipboard.setData('text/plain', 'First Line\nSecond Line');
    document.activeElement?.dispatchEvent(new ClipboardEvent('paste', { clipboardData: clipboard, bubbles: true, cancelable: true }));
  });
  await page.keyboard.type('!');
  await expect(page.locator('.docx-page-content').getByText('Start First Line', { exact: true })).toBeVisible();
  await expect(page.locator('.docx-page-content').getByText('Second Line!', { exact: true })).toBeVisible();
});

test('Continues Typing After Rich Text Pasted In The Middle Of A Paragraph', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('BeforeAfter');
  await page.keyboard.press('Home');
  for (let index = 0; index < 6; index++) await page.keyboard.press('ArrowRight');
  await page.evaluate(() => {
    const clipboard = new DataTransfer();
    clipboard.setData('text/plain', 'Copied');
    clipboard.setData('text/html', '<p><strong>Copied</strong></p>');
    document.activeElement?.dispatchEvent(new ClipboardEvent('paste', { clipboardData: clipboard, bubbles: true, cancelable: true }));
  });
  await page.keyboard.type('!');
  await expect(page.locator('.docx-page-content').getByText('BeforeCopied!After', { exact: true })).toBeVisible();
});

test('Places The Cursor After Pasting Over A Backward Selection Without Jumping To Later Paragraphs', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('BeforeREMOVEAfter');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Unchanged Paragraph');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('Home');
  for (let index = 0; index < 12; index++) await page.keyboard.press('ArrowRight');
  for (let index = 0; index < 6; index++) await page.keyboard.press('Shift+ArrowLeft');
  await page.evaluate(() => {
    const clipboard = new DataTransfer();
    clipboard.setData('text/plain', 'Copied');
    clipboard.setData('text/html', '<p><strong>Copied</strong></p>');
    document.activeElement?.dispatchEvent(new ClipboardEvent('paste', { clipboardData: clipboard, bubbles: true, cancelable: true }));
  });
  await page.keyboard.type('!');
  await expect(page.locator('.docx-page-content').getByText('BeforeCopied!After', { exact: true })).toBeVisible();
  await expect(page.locator('.docx-page-content').getByText('Unchanged Paragraph', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+z');
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').getByText('BeforeREMOVEAfter', { exact: true })).toBeVisible();
});

test('Inserts Editable Math From The Insert Menu And Can Undo It', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Math Equation', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Insert Math Equation', exact: true });
  await dialog.getByRole('textbox', { name: 'Math Expression', exact: true }).fill('x^2 + y^2 = z^2');
  await dialog.getByRole('button', { name: 'Insert Equation', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('oMath');
  expect(xml).toContain('sSup');
  await page.getByRole('button', { name: 'Undo (Ctrl+Z)', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const undone = await page.evaluate(() => window.__nativeTest.saved);
  expect(strFromU8(unzipSync(new Uint8Array(undone!))['word/document.xml'])).not.toContain('<m:oMath');
});

test('Pastes Single Paragraph HTML Inline In The Middle Of A Word', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('wordminute');
  await page.keyboard.press('Home');
  for (let index = 0; index < 4; index++) await page.keyboard.press('ArrowRight');
  await page.evaluate(() => {
    const clipboard = new DataTransfer();
    clipboard.setData('text/plain', ' per \r\n');
    clipboard.setData('text/html', '<html><body><!--StartFragment--><p><b> per </b></p><p><br></p><!--EndFragment--></body></html>');
    document.activeElement?.dispatchEvent(new ClipboardEvent('paste', { clipboardData: clipboard, bubbles: true, cancelable: true }));
  });
  await page.keyboard.type('next ');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml.match(/<w:p[\s>]/g)).toHaveLength(1);
  expect(xml).toMatch(/<w:b[\s/>]/);
  await expect(page.locator('.docx-page-content').getByText('word per next minute', { exact: true })).toBeVisible();
});

test('Edits Footers And Saves Live Page Number Fields', async ({ page }) => {
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Headers And Footers', exact: true }).hover();
  await page.getByRole('menuitem', { name: 'Edit Footer', exact: true }).click();
  await expect(page.locator('.header-footer-strip')).toContainText('Editing Footer');
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Page Number Fields', exact: true }).hover();
  await page.getByRole('menuitem', { name: 'Insert Page X Of Y', exact: true }).click();
  await page.getByRole('button', { name: 'Return To Document', exact: true }).click();
  await expect(page.locator('.header-footer-strip')).not.toBeVisible();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const parts = unzipSync(new Uint8Array(saved!));
  const footer = Object.entries(parts).filter(([name]) => /word\/footer[^/]*\.xml$/.test(name)).map(([, bytes]) => strFromU8(bytes)).join('');
  expect(footer).toContain('PAGE');
  expect(footer).toContain('NUMPAGES');
});

test('Exports Superscript Formatting And Toggles Paragraph Marks Without Editing Content', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Exponent');
  await page.keyboard.press('Control+a');
  await page.getByRole('button', { name: 'Menus', exact: true }).click();
  await page.getByRole('textbox', { name: 'Search The Menus' }).fill('superscript');
  await page.getByRole('button', { name: 'Superscript', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('superscript');
  expect(xml).toContain('Exponent');
  await page.getByRole('menuitem', { name: 'View', exact: true }).click();
  await page.getByRole('menuitemcheckbox', { name: 'Show Paragraph Breaks', exact: true }).click();
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
});

test('Inserts Dates And Searchable Symbols Into The Saved DOCX', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Created: ');
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Date And Time', exact: true }).click();
  const dateDialog = page.getByRole('dialog', { name: 'Insert Date And Time', exact: true });
  const date = await dateDialog.locator('.date-formats label').first().innerText();
  await dateDialog.getByRole('button', { name: 'Insert Date And Time', exact: true }).click();
  await expect(dateDialog).not.toBeVisible();
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Special Characters', exact: true }).click();
  const symbols = page.getByRole('dialog', { name: 'Special Characters', exact: true });
  await symbols.getByRole('textbox').fill('copyright');
  await expect(symbols.locator('.symbol-grid button')).toHaveCount(1);
  await symbols.getByRole('button', { name: 'Insert Copyright', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Created: ');
  expect(xml).toContain(date.trim());
  expect(xml).toContain('©');
});

test('Validates And Inserts A Custom Table Larger Than The Quick Picker', async ({ page }) => {
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Custom Table', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Insert Custom Table', exact: true });
  await dialog.getByRole('spinbutton', { name: 'Rows', exact: true }).fill('0');
  await dialog.getByRole('button', { name: 'Insert Table', exact: true }).click();
  await expect(dialog.getByRole('alert')).toHaveText('Rows And Columns Must Be Whole Numbers From 1 To 30.');
  await dialog.getByRole('spinbutton', { name: 'Rows', exact: true }).fill('7');
  await dialog.getByRole('spinbutton', { name: 'Columns', exact: true }).fill('2');
  await dialog.getByRole('button', { name: 'Insert Table', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml.match(/<w:tr[\s>]/g)).toHaveLength(7);
  expect(xml.match(/<w:tc[\s>]/g)).toHaveLength(14);
});

test('Persists Ruler Units And Writing Goals With A Darker Theme', async ({ page }) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await page.getByRole('combobox', { name: 'Ruler Units', exact: true }).selectOption('cm');
  await page.getByRole('spinbutton', { name: 'Word Goal', exact: true }).fill('10');
  await page.screenshot({ path: 'artifacts/ui/08-midnight-settings.png' });
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.app-shell')).toHaveCSS('background-color', 'rgb(17, 19, 24)');
  await expect(page.locator('.docx-page').first()).toHaveCSS('background-color', 'rgb(255, 255, 255)');
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('One Two Three');
  await expect(page.getByRole('button', { name: 'Word Count', exact: true })).toHaveText('3 / 10 Words');
  await page.getByRole('button', { name: 'Word Count', exact: true }).click();
  const statistics = page.getByRole('dialog', { name: 'Word Count', exact: true });
  await expect(statistics.getByText('30%', { exact: true })).toBeVisible();
  await expect(statistics.getByText('7 Words To Go.', { exact: true })).toBeVisible();
  await expect(statistics.getByText('1 Min', { exact: true })).toBeVisible();
  await statistics.getByRole('button', { name: 'Done', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  await page.reload();
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Ruler Units', exact: true })).toHaveValue('cm');
  await expect(page.getByRole('spinbutton', { name: 'Word Goal', exact: true })).toHaveValue('10');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.screenshot({ path: 'artifacts/ui/08-midnight-editor.png' });
});

test('Searches Menus And Keeps Formatting Available In A Compact Toolbar', async ({ page }) => {
  await page.getByRole('button', { name: 'Menus', exact: true }).click();
  const search = page.getByRole('dialog', { name: 'Search The Menus', exact: true });
  await search.getByRole('textbox').fill('settings');
  await search.getByRole('textbox').press('Enter');
  await expect(search).not.toBeVisible();
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveCSS('border-top-color', 'rgb(56, 63, 75)');
  await expect(page.getByRole('combobox', { name: 'Font Size', exact: true })).toHaveCSS('border-top-width', '1px');
  await page.screenshot({ path: 'artifacts/ui/06-docs-dark.png' });
  await page.keyboard.press('Alt+/');
  await expect(search).toBeVisible();
  await search.getByRole('textbox').fill('Nothing Matches This');
  await expect(search.getByText('No Matching Tools. Try Another Search.')).toBeVisible();
  await page.keyboard.press('Escape');
  await page.setViewportSize({ width: 800, height: 900 });
  await expect(page.getByRole('button', { name: 'More', exact: true })).toBeVisible();
  const toolbar = page.locator('.toolbar-shell');
  await expect.poll(() => toolbar.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
  await expect.poll(() => toolbar.evaluate((element) => element.clientHeight)).toBe(44);
  await page.getByRole('button', { name: 'More', exact: true }).click();
  await page.getByRole('button', { name: 'Bold (Ctrl+B)', exact: true }).click();
  await page.keyboard.press('Escape');
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Compact Toolbar Formatting');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Compact Toolbar Formatting');
  expect(xml).toMatch(/<w:b[\s/>]/);
});

test('Uses Inter Outside The Document And Preserves Authored Colors In Dark Mode', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, coloredFixture());
  await fileAction(page, 'Open');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  const authoredText = page.locator('.docx-page-content').getByText('Original Mixed CASE Content', { exact: true }).locator('.layout-run-text').first();
  await expect(authoredText).toHaveCSS('color', 'rgb(198, 32, 38)');
  for (const selector of ['.app-header', '.docs-toolbar', '.document-outline', '.status-bar', '.ruler-wrap']) {
    await expect(page.locator(selector).first()).toHaveCSS('font-family', /Inter/);
  }
  await page.screenshot({ path: 'test-results/inter-light.png' });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.docx-page').first()).toHaveCSS('background-color', 'rgb(255, 255, 255)');
  await expect(page.locator('.docx-page-content').first()).toHaveCSS('filter', 'none');
  await expect(authoredText).toHaveCSS('color', 'rgb(198, 32, 38)');
  await page.screenshot({ path: 'test-results/inter-dark.png' });
  await page.keyboard.press('Control+s');
  await expect(page.locator('main')).toHaveAttribute('aria-busy', 'false');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).not.toContain('w:val="Inter"');
  expect(xml).toContain('Original Mixed CASE Content');
});

test('Exports Plain Text Without Marking The DOCX As Saved', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Keep CASE Café');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Second Line');
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Export Plain Text', exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.__nativeTest.exported)).toBe('Keep CASE Café\nSecond Line');
  await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
  await expect(page.getByRole('status')).toContainText('Exported Export.txt');
  await page.getByRole('button', { name: 'Dismiss Notification', exact: true }).click();
  await expect(page.locator('.workspace-notice')).toHaveCount(0);
});

test('Keeps Focus Mode When Closing Find And Refocuses Find On Repeated Shortcuts', async ({ page }) => {
  await page.keyboard.press('Control+Shift+f');
  await expect(page.locator('.app-header')).not.toBeVisible();
  await page.keyboard.press('Control+f');
  await expect(page.getByRole('textbox', { name: 'Find Text', exact: true })).toBeFocused();
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.press('Control+f');
  await expect(page.getByRole('textbox', { name: 'Find Text', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('region', { name: 'Find And Replace', exact: true })).toHaveCount(0);
  await expect(page.locator('.app-header')).not.toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('.app-header')).toBeVisible();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.keyboard.press('Control+n');
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).not.toBeVisible();
});

test('Navigates To A Page And Shows The Shortcut Reference', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, fixture());
  await fileAction(page, 'Open');
  await expect(page.locator('.docx-page')).toHaveCount(2);
  await page.keyboard.press('Control+g');
  await page.getByRole('spinbutton', { name: 'Page Number', exact: true }).fill('9');
  await page.getByRole('dialog', { name: 'Go To Page', exact: true }).getByRole('button', { name: 'Go To Page', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Choose A Page From 1 To 2');
  await page.getByRole('spinbutton', { name: 'Page Number', exact: true }).fill('2');
  await page.getByRole('dialog', { name: 'Go To Page', exact: true }).getByRole('button', { name: 'Go To Page', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Go To Page', exact: true })).not.toBeVisible();
  await page.keyboard.press('Control+/');
  await expect(page.getByRole('dialog', { name: 'Keyboard Shortcuts', exact: true })).toBeVisible();
  await expect(page.getByRole('dialog').getByText('Ctrl+Shift+I', { exact: true })).toBeVisible();
});

test('Reopens A Recent Document With Unsaved Changes Protection', async ({ page }) => {
  await page.evaluate((bytes) => {
    window.__nativeTest.openBytes = bytes;
    window.__nativeTest.recent = [{ token: 'recent', name: 'Recent.docx', path: 'D:/Documents/Recent.docx', available: true }];
    window.dispatchEvent(new Event('insyndoc:recent-changed'));
  }, fixture());
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Keep My Work');
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Open Recent', exact: true }).hover();
  await page.getByRole('menuitem', { name: /^Recent.docx/ }).click();
  await expect(page.getByRole('dialog', { name: 'Save Your Changes?', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Untitled.docx', exact: true })).toBeVisible();
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Open Recent', exact: true }).hover();
  await page.getByRole('menuitem', { name: /^Recent.docx/ }).click();
  await page.getByRole('button', { name: 'Discard Changes', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Recent.docx', exact: true })).toBeVisible();
});

test('Edits And Saves DOCX While Preserving A Session Across Views', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.screenshot({ path: 'test-results/editor-initial.png' });
  const content = page.locator('.docx-page-content').first();
  await content.click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Hello Mixed CASE');
  await expect(page.getByText('Unsaved Changes', { exact: true }).first()).toBeVisible();
  await switchView(page, 'Continuous');
  await expect(page.locator('.editor-workspace')).toHaveClass(/view-continuous/);
  await expect(page.locator('.docx-page').first()).toHaveCSS('box-shadow', 'none');
  await switchView(page, 'Separate Pages');
  await page.getByRole('button', { name: 'Undo (Ctrl+Z)', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Redo (Ctrl+Y)', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: 'Redo (Ctrl+Y)', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  expect(saved).not.toBeNull();
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  await writeFile('test-results/Edited.docx', new Uint8Array(saved!));
  expect(xml).toContain('Hello Mixed CASE');
  expect(errors).toEqual([]);
});

test('Exports Text Formatting And Headings As Word Properties', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Styled Mixed Case');
  await page.keyboard.press('Control+a');
  await page.getByRole('button', { name: 'Bold (Ctrl+B)', exact: true }).click();
  await page.getByRole('button', { name: 'Italic (Ctrl+I)', exact: true }).click();
  await page.getByRole('button', { name: 'Underline (Ctrl+U)', exact: true }).click();
  await page.getByRole('combobox', { name: 'Font Size', exact: true }).fill('18');
  await page.getByRole('combobox', { name: 'Font Size', exact: true }).press('Enter');
  await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
  await page.getByRole('option', { name: 'Heading 1', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Styled Mixed Case');
  expect(xml).toMatch(/<w:b[\s/>]/);
  expect(xml).toMatch(/<w:i[\s/>]/);
  expect(xml).toMatch(/<w:u[\s/>]/);
  expect(xml).toContain('w:val="52"');
  expect(xml).toContain('Heading1');
  await expect(page.getByRole('navigation', { name: 'Headings', exact: true }).getByRole('button', { name: 'Styled Mixed Case', exact: true })).toBeVisible();
  await writeFile('test-results/Formatted.docx', new Uint8Array(saved!));
});

test('Persists Dark Mode And Keeps Saved Document Colors Unchanged', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Theme Should Not Edit My Document');
  await page.keyboard.press('Control+s');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  const before = await page.evaluate(() => window.__nativeTest.saved);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await expect(page.locator('.app-shell')).toHaveAttribute('data-theme', 'dark');
  await page.getByRole('switch', { name: 'Show Ruler', exact: false }).uncheck();
  await page.getByRole('switch', { name: 'Show Document Outline', exact: false }).uncheck();
  await page.screenshot({ path: 'test-results/settings-dark.png' });
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.ruler-wrap')).toHaveCount(0);
  await expect(page.getByRole('complementary', { name: 'Document Outline', exact: true })).toHaveCount(0);
  await page.keyboard.press('Control+s');
  await expect(page.locator('main')).toHaveAttribute('aria-busy', 'false');
  const after = await page.evaluate(() => window.__nativeTest.saved);
  expect(strFromU8(unzipSync(new Uint8Array(after!))['word/document.xml']))
    .toEqual(strFromU8(unzipSync(new Uint8Array(before!))['word/document.xml']));
  await page.reload();
  await expect(page.locator('.app-shell')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('.docx-page').first()).toBeVisible();
  await expect(page.locator('.ruler-wrap')).toHaveCount(0);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'System', exact: true }).click();
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('.app-shell')).toHaveAttribute('data-theme', 'light');
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('.app-shell')).toHaveAttribute('data-theme', 'dark');
});

test('Replaces Whole Words In One Undo Step And Reports Word Count', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Blue blueprint Blue');
  await expect(page.getByRole('button', { name: 'Word Count', exact: true })).toHaveText('3 Words');
  await page.keyboard.press('Control+Shift+i');
  await page.getByRole('textbox', { name: 'Find Text', exact: true }).fill('Blue');
  await page.getByRole('checkbox', { name: 'Whole Words', exact: true }).check();
  await expect(page.locator('.match-count')).toHaveText('2');
  await page.getByRole('textbox', { name: 'Replace With', exact: true }).fill('Azure');
  await page.getByRole('button', { name: 'Replace All', exact: true }).click();
  await expect(page.locator('.match-count')).toHaveText('0');
  await page.getByRole('button', { name: 'Close Find', exact: true }).click();
  await page.getByRole('button', { name: 'Undo (Ctrl+Z)', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Blue blueprint Blue');
  expect(xml).not.toContain('Azure');
  await page.getByRole('button', { name: 'Redo (Ctrl+Y)', exact: true }).click();
  await page.getByRole('button', { name: 'Word Count', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Word Count', exact: true })).toBeVisible();
});

test('Changes Zoom And Uses Focus Mode Without Remounting The Editor', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Stay In The Document');
  const pageBefore = await page.locator('.docx-page').first().boundingBox();
  await page.getByRole('button', { name: /^Zoom Level:/ }).click();
  await page.getByRole('option', { name: '125%', exact: true }).click();
  await expect.poll(async () => (await page.locator('.docx-page').first().boundingBox())!.width).toBeGreaterThan(pageBefore!.width);
  await page.keyboard.press('Control+Shift+f');
  await expect(page.locator('.app-header')).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Exit Focus Mode', exact: false })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('.app-header')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Undo (Ctrl+Z)', exact: true })).toBeEnabled();
});

test('Inserts Tables And Local Images Into A Valid DOCX Package', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Table', exact: true }).hover();
  await page.getByLabel('2 × 2', { exact: true }).click();
  await page.keyboard.type('Table Cell');
  const png = await page.evaluate(() => {
    const canvas = document.createElement('canvas');
    canvas.width = 64;
    canvas.height = 32;
    const context = canvas.getContext('2d')!;
    context.fillStyle = '#205b43';
    context.fillRect(0, 0, 64, 32);
    return canvas.toDataURL('image/png').split(',')[1];
  });
  const choosingImage = page.waitForEvent('filechooser');
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Image', exact: true }).click();
  await (await choosingImage).setFiles({
    name: 'Pixel.png', mimeType: 'image/png',
    buffer: Buffer.from(png, 'base64')
  });
  await expect(page.locator('.docx-page img').first()).toBeVisible();
  await page.keyboard.press('Control+s');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const parts = unzipSync(new Uint8Array(saved!));
  const xml = strFromU8(parts['word/document.xml']);
  expect(xml).toContain('<w:tbl>');
  expect(xml).toContain('Table Cell');
  expect(xml).toContain('w:drawing');
  expect(Object.keys(parts).some((name) => name.startsWith('word/media/'))).toBe(true);
  await writeFile('test-results/Table And Image.docx', new Uint8Array(saved!));
});

test('Opens Multipage DOCX And Refuses Corrupt Input Without Losing Content', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, fixture());
  await fileAction(page, 'Open');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  await expect(page.locator('.docx-page')).toHaveCount(2);
  await page.evaluate(() => { window.__nativeTest.openBytes = [80, 75, 3, 4, 0, 0]; });
  await fileAction(page, 'Open');
  await expect(page.getByRole('alert')).toContainText('Could Not Be Opened');
  await expect(page.locator('.docx-page')).toHaveCount(2);
  await page.keyboard.press('Control+s');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  expect(strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml'])).toContain('Original Mixed CASE Content');
});

test('Labels The Footer View Toggle And Announces Its State', async ({ page }) => {
  const toggle = page.getByRole('button', { name: 'Continuous View', exact: true });
  await expect(toggle).toHaveAttribute('aria-pressed', 'false');
  await expect(toggle).toHaveText('View: Separate Pages');
  await toggle.click();
  await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  await expect(toggle).toHaveText('View: Continuous');
  await expect(page.locator('.editor-workspace')).toHaveClass(/view-continuous/);
  await toggle.click();
  await expect(toggle).toHaveAttribute('aria-pressed', 'false');
});

test('Keeps Unsaved Work And Editing Available After Unsupported And Malformed Opens', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Keep Unsaved Mixed CASE');
  const invalidDocuments = [
    [80, 75, 3, 4, 0, 0],
    Array.from(zipSync({ 'unrelated.txt': strToU8('A ZIP Is Not A DOCX') }))
  ];
  for (const bytes of invalidDocuments) {
    await page.evaluate((incoming) => { window.__nativeTest.openBytes = incoming; }, bytes);
    await fileAction(page, 'Open');
    await page.getByRole('button', { name: 'Discard Changes', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('Could Not Be Opened');
    await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
    await expect(page.getByRole('heading', { name: 'Untitled.docx', exact: true })).toBeVisible();
    await expect(page.locator('.docx-page-content').first()).toContainText('Keep Unsaved Mixed CASE');
    await expect(page.getByRole('button', { name: 'Undo (Ctrl+Z)', exact: true })).toBeEnabled();
  }
  await page.locator('.docx-page-content').first().click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type(' Still Editable');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Keep Unsaved Mixed CASE');
  expect(xml).toContain('Still Editable');
});

test('Keeps Changes After Cancelled And Failed Saves And Prompts Before New', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Keep These Words');
  await page.evaluate(() => { window.__nativeTest.cancelSave = true; });
  await page.keyboard.press('Control+s');
  await expect(page.getByText('Unsaved Changes', { exact: true }).first()).toBeVisible();
  await page.evaluate(() => { window.__nativeTest.failSave = true; });
  await page.keyboard.press('Control+s');
  await expect(page.getByRole('alert')).toContainText('Could Not Replace');
  await fileAction(page, 'New');
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await fileAction(page, 'New');
  await page.getByRole('button', { name: 'Discard Changes', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Untitled.docx', exact: true })).toBeVisible();
  await expect(page.locator('.document-save-indicator')).toHaveText('New Document');
});

test('Loads Additional Fonts On Demand And Saves Their Families Without External Requests', async ({ page }) => {
  const external: string[] = [];
  const loaded = new Set<string>();
  const failed: string[] = [];
  await page.route('**/*', async (route) => {
    const url = new URL(route.request().url());
    if (url.hostname !== '127.0.0.1') { external.push(url.href); await route.abort(); return; }
    await route.continue();
  });
  page.on('response', (response) => {
    if (!response.url().includes('/document-fonts/')) return;
    if (response.status() >= 400) failed.push(response.url());
    else loaded.add(response.url());
  });
  await page.reload();
  await expect(page.locator('.docx-page').first()).toBeVisible();
  expect(loaded.size).toBe(0);
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  for (let index = 0; index < additionalFontFamilies.length; index++) {
    const family = additionalFontFamilies[index];
    await page.getByRole('button', { name: /^Select Font Family:/ }).click();
    await page.getByRole('option', { name: family, exact: true }).click();
    await expect.poll(() => loaded.size).toBe((index + 1) * 4);
    await page.keyboard.type(`${family} Font Sample`);
    await page.keyboard.press('Enter');
  }
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  for (const family of additionalFontFamilies) {
    expect(xml).toContain(family);
    expect(xml).toContain(`${family} Font Sample`);
  }
  expect(external).toEqual([]);
  expect(failed).toEqual([]);
  await page.screenshot({ path: 'artifacts/ui/15-additional-document-fonts.png' });
});

test('Loads Packaged Fonts And Edits Without External Services', async ({ page, context }) => {
  const external: string[] = [];
  const failures: string[] = [];
  const fonts: string[] = [];
  await page.route('**/*', async (route) => {
    const url = new URL(route.request().url());
    if (url.hostname !== '127.0.0.1') {
      external.push(url.href);
      await route.abort();
      return;
    }
    await route.continue();
  });
  page.on('response', (response) => {
    if (response.status() >= 400) failures.push(response.url());
    if (/\.ttf$/.test(response.url())) fonts.push(response.url());
  });
  await page.reload();
  await expect(page.locator('.docx-page').first()).toBeVisible();
  await expect.poll(() => fonts.length).toBeGreaterThanOrEqual(4);
  await context.setOffline(true);
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Offline Words');
  await page.keyboard.press('Control+s');
  await expect(page.getByRole('heading', { name: 'Example.docx', exact: true })).toBeVisible();
  expect(external).toEqual([]);
  expect(failures).toEqual([]);
});


test('Uses The Left Outline And Finds Formatting Tools Without Footer Badges', async ({ page }) => {
  await expect(page.getByText('All Files Stay On This Device', { exact: true })).toHaveCount(0);
  await expect(page.locator('.file-format')).toHaveCount(0);
  const outline = await page.getByRole('complementary', { name: 'Document Outline', exact: true }).boundingBox();
  const stage = await page.locator('.document-stage').boundingBox();
  expect(outline).not.toBeNull();
  expect(stage).not.toBeNull();
  expect(outline!.x + outline!.width).toBeLessThanOrEqual(stage!.x + 1);
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Tool Finder Formatting');
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Alt+/');
  const search = page.getByRole('dialog', { name: 'Search The Menus', exact: true });
  await search.getByRole('textbox').fill('Bold');
  await search.getByRole('button', { name: 'Bold Ctrl+B', exact: true }).click();
  await expect(search).not.toBeVisible();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Tool Finder Formatting');
  expect(xml).toMatch(/<w:b[\s/>]/);
  const painter = page.getByRole('button', { name: /^Format Painter / });
  await expect(painter).toBeVisible();
  await expect(painter).toBeEnabled();
  await painter.click();
  await page.keyboard.press('Escape');
  await page.screenshot({ path: 'artifacts/ui/docs-workspace-verified.png' });
});


test('Keeps The Caret When Applying Formatting Through Tool Finder', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Alpha');
  await page.keyboard.press('Alt+/');
  const search = page.getByRole('dialog', { name: 'Search The Menus', exact: true });
  await search.getByRole('textbox').fill('Bold');
  await search.getByRole('textbox').press('Enter');
  await expect(search).not.toBeVisible();
  await page.keyboard.type('Beta');
  await expect(page.locator('.docx-page-content').first()).toHaveText('AlphaBeta');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toMatch(/<w:b[\s/>]/);
});


test('Applies Paint Format To A Target And Undoes Only The Target Change', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Source');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Target');
  await page.locator('.layout-run-text').getByText('Source', { exact: true }).dblclick();
  await page.getByRole('button', { name: 'Bold (Ctrl+B)', exact: true }).click();
  await page.getByRole('button', { name: /^Format Painter / }).click();
  const target = await page.locator('.layout-run-text').getByText('Target', { exact: true }).boundingBox();
  expect(target).not.toBeNull();
  await page.mouse.move(target!.x + 1, target!.y + target!.height / 2);
  await page.mouse.down();
  await page.mouse.move(target!.x + target!.width - 1, target!.y + target!.height / 2, { steps: 8 });
  await page.mouse.up();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const painted = await page.evaluate(() => window.__nativeTest.saved);
  const paintedXml = strFromU8(unzipSync(new Uint8Array(painted!))['word/document.xml']);
  expect(paintedXml).toMatch(/<w:r>(?:(?!<w:r>).)*<w:b[\s/>](?:(?!<w:r>).)*Target/s);
  await page.keyboard.press('Control+z');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const undone = await page.evaluate(() => window.__nativeTest.saved);
  const undoneXml = strFromU8(unzipSync(new Uint8Array(undone!))['word/document.xml']);
  expect(undoneXml).not.toMatch(/<w:r>(?:(?!<w:r>).)*<w:b[\s/>](?:(?!<w:r>).)*Target/s);
  expect(undoneXml).toMatch(/<w:r>(?:(?!<w:r>).)*<w:b[\s/>](?:(?!<w:r>).)*Source/s);
});

for (const note of ['Footnote', 'Endnote'] as const) {
  test(`Exports An Inserted ${note} Reference And Note Part`, async ({ page }) => {
    await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
    await page.keyboard.type('Body With A Note');
    await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
    await page.getByRole('menuitem', { name: `Insert ${note}`, exact: true }).click();
    await page.keyboard.press('Control+s');
    await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
    const saved = await page.evaluate(() => window.__nativeTest.saved);
    const parts = unzipSync(new Uint8Array(saved!));
    const kind = note.toLowerCase();
    expect(strFromU8(parts['word/document.xml'])).toContain(`${kind}Reference`);
    expect(parts[`word/${kind}s.xml`]).toBeDefined();
  });
}

test('Navigates Tool Finder Results With Arrow Keys', async ({ page }) => {
  await page.keyboard.press('Alt+/');
  const search = page.getByRole('dialog', { name: 'Search The Menus', exact: true });
  await search.getByRole('textbox').fill('Align');
  await search.getByRole('textbox').press('ArrowDown');
  await expect(search.getByRole('button', { name: 'Align Left', exact: true })).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(search.getByRole('button', { name: 'Align Center', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(search).not.toBeVisible();
});


test('Creates Navigable Headings And Restores The Outline After Focus Mode', async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('First Heading');
  await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
  await page.getByRole('option', { name: 'Heading 1', exact: true }).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Control+Enter');
  await page.keyboard.type('Second Heading');
  await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
  await page.getByRole('option', { name: 'Heading 1', exact: true }).click();
  const outline = page.getByRole('complementary', { name: 'Document Outline', exact: true });
  await page.getByRole('button', { name: 'Toggle Document Outline', exact: true }).click();
  await expect(outline.getByRole('button', { name: 'First Heading', exact: true })).toBeVisible();
  await expect(outline.getByRole('button', { name: 'Second Heading', exact: true })).toBeVisible();
  await outline.getByRole('button', { name: 'First Heading', exact: true }).click();
  await expect(outline).not.toBeVisible();
  await page.getByRole('button', { name: 'Toggle Document Outline', exact: true }).click();
  await expect(outline.getByRole('button', { name: 'First Heading', exact: true })).toHaveAttribute('aria-current', 'location');
  await page.keyboard.press('Control+Shift+f');
  await expect(outline).not.toBeVisible();
  await page.keyboard.press('Escape');
  await expect(outline).toBeVisible();
  await outline.getByRole('button', { name: 'Hide Outline', exact: true }).click();
  await expect(outline).not.toBeVisible();
  await page.getByRole('button', { name: 'Toggle Document Outline', exact: true }).click();
  await expect(outline).toBeVisible();
});

test('Adds A Bookmark And Preserves It In The Saved DOCX', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Bookmarked Text');
  await page.keyboard.press('Control+a');
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: /^Bookmark/ }).click();
  const dialog = page.getByRole('dialog', { name: 'Bookmark', exact: true });
  await dialog.getByRole('textbox', { name: /Bookmark name/i }).fill('ImportantText');
  await dialog.getByRole('button', { name: 'Add', exact: true }).click();
  if (await dialog.isVisible()) await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('bookmarkStart');
  expect(xml).toContain('ImportantText');
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: /^Bookmark/ }).click();
  await expect(dialog.getByRole('button', { name: 'Go To', exact: true })).toBeEnabled();
  await dialog.getByRole('button', { name: 'Go To', exact: true }).click();
});

test('Applies Paragraph Spacing And Saves The Paragraph Properties', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Spaced Paragraph');
  await page.getByRole('menuitem', { name: 'Format', exact: true }).click();
  await page.getByRole('menuitem', { name: /Line Spacing Options/ }).click();
  const dialog = page.getByRole('dialog', { name: 'Paragraph', exact: true });
  await dialog.getByLabel('After', { exact: true }).fill('12');
  await dialog.getByRole('button', { name: 'OK', exact: true }).click();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toMatch(/<w:spacing[^>]*w:after="240"/);
});


test('Builds A Table Of Contents From Headings And Preserves It When Reopened', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Chapter One');
  await page.getByRole('combobox', { name: 'Paragraph Style', exact: true }).click();
  await page.getByRole('option', { name: 'Heading 1', exact: true }).click();
  await page.keyboard.press('Control+Home');
  await page.getByRole('menuitem', { name: 'Insert', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Table Of Contents', exact: true }).click();
  await expect.poll(async () => (await page.locator('.docx-page-content').allTextContents()).join('').match(/Chapter One/g)?.length ?? 0).toBe(2);
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('TOC');
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, saved);
  await fileAction(page, 'Open');
  await expect.poll(async () => (await page.locator('.docx-page-content').allTextContents()).join('').match(/Chapter One/g)?.length ?? 0).toBe(2);
});


for (const clipboardCase of [
  { name: 'Plain Text', text: 'Copied', html: null, expected: 'BeforeCopied!After' },
  { name: 'Rich Text', text: 'Copied', html: '<p><strong>Copied</strong></p>', expected: 'BeforeCopied!After' },
  { name: 'Multiline Rich Text', text: 'First\nSecond', html: '<p>First</p><p>Second</p>', expected: 'BeforeFirstSecond!After' }
]) {
  test(`Keeps The Cursor After ${clipboardCase.name} Pasted From The System Clipboard`, async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
    await page.keyboard.type('BeforeAfter');
    await page.keyboard.press('Home');
    for (let index = 0; index < 6; index++) await page.keyboard.press('ArrowRight');
    await page.evaluate(async ({ text, html }) => {
      const formats: Record<string, Blob> = { 'text/plain': new Blob([text], { type: 'text/plain' }) };
      if (html) formats['text/html'] = new Blob([html], { type: 'text/html' });
      await navigator.clipboard.write([new ClipboardItem(formats)]);
    }, clipboardCase);
    await page.keyboard.press('Control+v');
    await page.keyboard.type('!');
    await expect.poll(async () => (await page.locator('.docx-page-content').allTextContents()).join('')).toBe(clipboardCase.expected);
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Control+z');
    await expect(page.locator('.docx-page-content').first()).toHaveText('BeforeAfter');
  });
}


test('Pastes A Copied Sentence Inline Despite A Covered Paragraph End', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Copied sentence.');
  await page.keyboard.press('End');
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Control+c');
  await page.evaluate(async () => {
    const item = (await navigator.clipboard.read())[0];
    const text = await (await item.getType('text/plain')).text();
    const html = (await (await item.getType('text/html')).text()).replace('data-docx-fragment-end="open"', 'data-docx-fragment-end="covered"');
    await navigator.clipboard.write([new ClipboardItem({
      'text/plain': new Blob([text], { type: 'text/plain' }),
      'text/html': new Blob([html], { type: 'text/html' })
    })]);
  });
  await page.keyboard.press('Control+End');
  for (let index = 0; index < 3; index++) await page.keyboard.press('Control+v');
  await page.keyboard.type('!');
  await expect(page.locator('.docx-page-content').first()).toHaveText('Copied sentence.'.repeat(4) + '!');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml.match(/<w:p[\s>]/g)).toHaveLength(1);
});

test('Pastes A Single Rich Paragraph Inline Without Empty Clipboard Paragraphs', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('BeforeAfter');
  await page.keyboard.press('Home');
  for (let index = 0; index < 6; index++) await page.keyboard.press('ArrowRight');
  await page.evaluate(async () => {
    await navigator.clipboard.write([new ClipboardItem({
      'text/plain': new Blob(['Copied\r\n\r\n'], { type: 'text/plain' }),
      'text/html': new Blob(['<div><p><strong>Copied</strong></p><p><br></p></div>'], { type: 'text/html' })
    })]);
  });
  await page.keyboard.press('Control+v');
  await page.keyboard.type('!');
  await expect(page.locator('.docx-page-content').first()).toHaveText('BeforeCopied!After');
  await page.keyboard.press('Control+s');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml.match(/<w:p[\s>]/g)).toHaveLength(1);
  expect(xml).toMatch(/<w:b[\s/>]/);
});


test('Offers Close Without Saving And Closes Without Writing Unsaved Changes', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Unsaved Work');
  await expect.poll(() => page.evaluate(() => Boolean(window.__nativeTest.requestClose))).toBe(true);
  await page.evaluate(() => window.__nativeTest.requestClose?.());
  const prompt = page.getByRole('dialog', { name: 'Save Your Changes?', exact: true });
  await expect(prompt.getByRole('button', { name: 'Close Without Saving', exact: true })).toBeVisible();
  await prompt.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(prompt).not.toBeVisible();
  expect(await page.evaluate(() => window.__nativeTest.closed)).toBe(false);
  await expect(page.locator('.document-save-indicator')).toHaveText('Unsaved Changes');
  await page.evaluate(() => window.__nativeTest.requestClose?.());
  await prompt.getByRole('button', { name: 'Close Without Saving', exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.__nativeTest.closed)).toBe(true);
  expect(await page.evaluate(() => window.__nativeTest.saveRequests)).toEqual([]);
});

test('Docks Find Above The Document And Keeps Replacement Keyboard Accessible', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Blue Blue');
  await page.keyboard.press('Control+Shift+i');
  const find = page.getByRole('region', { name: 'Find And Replace', exact: true });
  await page.getByRole('textbox', { name: 'Find Text', exact: true }).fill('Blue');
  await expect(find.locator('.match-count')).toHaveText('2');
  await page.getByRole('button', { name: 'Next Match', exact: true }).click();
  await expect(find.locator('.match-count')).toHaveText('1 Of 2');
  const panelBounds = await find.boundingBox();
  const viewportBounds = await page.locator('.document-viewport').boundingBox();
  expect(panelBounds!.y + panelBounds!.height).toBeLessThanOrEqual(viewportBounds!.y);
  const replacement = page.getByRole('textbox', { name: 'Replace With', exact: true });
  await replacement.fill('Green');
  await expect(replacement).toBeFocused();
  await expect(replacement).toHaveCSS('outline-style', 'solid');
  await replacement.press('Control+Enter');
  await expect(page.locator('.docx-page-content').first()).toContainText('Green Green');
});

test('Keeps The Canvas Wide With A Dismissible Outline Drawer And Footer Zoom', async ({ page }) => {
  await expect(page.locator('.status-zoom').getByRole('button', { name: /^Zoom Level:/ })).toBeVisible();
  await page.setViewportSize({ width: 800, height: 900 });
  const viewport = page.locator('.document-viewport');
  const outline = page.getByRole('complementary', { name: 'Document Outline', exact: true });
  await expect(outline).not.toBeVisible();
  await page.getByRole('button', { name: 'Toggle Document Outline', exact: true }).click();
  await expect(outline).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(outline).not.toBeVisible();
  await page.getByRole('button', { name: 'Toggle Document Outline', exact: true }).click();
  const openWidth = (await viewport.boundingBox())!.width;
  await page.screenshot({ path: 'artifacts/ui/council-outline-800.png' });
  await page.getByRole('button', { name: 'Close Document Outline', exact: true }).click({ position: { x: 700, y: 200 } });
  await expect(outline).toHaveCount(0);
  expect((await viewport.boundingBox())!.width).toBe(openWidth);
  await expect(page.locator('.document-save-indicator')).toBeVisible();
  await expect(page.locator('.document-save-indicator span')).toBeVisible();
  await page.keyboard.press('Control+f');
  await page.screenshot({ path: 'artifacts/ui/council-find-800.png' });
  await expect.poll(() => page.locator('.find-panel').evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
  await expect.poll(() => page.locator('.toolbar-shell').evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.keyboard.press('Escape');
  await page.screenshot({ path: 'artifacts/ui/council-workspace-1280.png' });
});

test('Opens Hyperlinks Through The Desktop Browser Launcher', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, hyperlinkFixture());
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await page.locator('a.docx-hyperlink').first().click({ position: { x: 4, y: 8 } });
  await page.getByTestId('hyperlink-popup-url').click();
  await expect.poll(() => page.evaluate(() => window.__nativeTest.openedLinks)).toEqual([{ url: 'https://example.com/', browserPath: null }]);
  await page.locator('a.docx-hyperlink').first().click({ modifiers: ['Control'] });
  await expect.poll(() => page.evaluate(() => window.__nativeTest.openedLinks.length)).toBe(2);
});

test('Preserves Hyperlink Destinations When Replacing All Link Labels', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, hyperlinkFixture());
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await page.keyboard.press('Control+Shift+i');
  await page.getByRole('textbox', { name: 'Find Text', exact: true }).fill('Hello World!');
  await expect(page.locator('.match-count')).toHaveText('2');
  await page.getByRole('textbox', { name: 'Replace With', exact: true }).fill('Example');
  await page.getByRole('button', { name: 'Replace All', exact: true }).click();
  await expect(page.locator('a.docx-hyperlink')).toHaveText('Example');
  await page.keyboard.press('Control+s');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const result = unzipSync(new Uint8Array(saved!));
  const xml = strFromU8(result['word/document.xml']);
  expect(xml).toMatch(/<w:hyperlink\b[^>]*>[\s\S]*?<w:t[^>]*>Example<\/w:t>[\s\S]*?<\/w:hyperlink>/);
  expect(strFromU8(result['word/_rels/document.xml.rels'])).toContain('https://example.com/');
  expect(xml).not.toContain('Hello World!');
  await page.getByRole('button', { name: 'Close Find', exact: true }).click();
  await page.keyboard.press('Control+z');
  await expect(page.locator('a.docx-hyperlink')).toHaveText('Hello World!');
  await expect(page.locator('.docx-page-content').first()).toContainText('Hello World! Hello World!');
});

test('Persists A Custom Link Browser And Can Return To The System Default', async ({ page }) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const browser = page.getByRole('combobox', { name: 'Link Browser', exact: true });
  await expect(browser).toHaveValue('system');
  await browser.selectOption('custom');
  await expect(browser).toHaveValue('system');
  await page.evaluate(() => { window.__nativeTest.browserChoice = 'C:/Program Files/Firefox/firefox.exe'; });
  await page.getByRole('button', { name: 'Choose Browser', exact: true }).click();
  await expect(browser).toHaveValue('custom');
  await expect(page.locator('.browser-path')).toHaveText('C:/Program Files/Firefox/firefox.exe');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.reload();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(browser).toHaveValue('custom');
  await page.getByRole('button', { name: 'Choose Browser', exact: true }).click();
  await expect(browser).toHaveValue('custom');
  await browser.selectOption('system');
  await expect(page.locator('.browser-path')).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('insyndoc.preferences.v1')!).browserPath)).toBeNull();
});

test('Uses The Selected Browser For Links And Reports Launch Failures', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, hyperlinkFixture());
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.evaluate(() => { window.__nativeTest.browserChoice = 'C:/Program Files/Browser/browser.exe'; });
  await page.getByRole('button', { name: 'Choose Browser', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Link Browser', exact: true })).toHaveValue('custom');
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.locator('a.docx-hyperlink').first().click({ modifiers: ['Control'] });
  await expect.poll(() => page.evaluate(() => window.__nativeTest.openedLinks)).toEqual([{ url: 'https://example.com/', browserPath: 'C:/Program Files/Browser/browser.exe' }]);
  await page.evaluate(() => { window.__nativeTest.failOpenLink = true; });
  await page.locator('a.docx-hyperlink').first().click({ modifiers: ['Control'] });
  await expect(page.getByRole('alert')).toContainText('Could Not Open The Selected Browser.');
  await expect(page.locator('a.docx-hyperlink')).toHaveText('Hello World!');
});

for (const replacement of [
  { label: 'Hello World!', query: 'Hello', text: 'Example', expected: 'Example World!' },
  { label: 'Hello World!', query: 'World!', text: 'Example', expected: 'Hello Example' },
  { label: 'X', query: 'X', text: 'Example', expected: 'Example' },
  { label: 'Hello Hello', query: 'Hello', text: 'Example', expected: 'Example Example' },
  { label: 'Hello \u{1F30D}', query: '\u{1F30D}', text: 'World', expected: 'Hello World' },
  { label: 'Hello World!', query: 'Hello World!', text: '', expected: '' }
]) {
  test(`Keeps Link Metadata In ${replacement.label} When Replacing ${replacement.query} With ${replacement.text || 'Empty Text'}`, async ({ page }) => {
    await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, hyperlinkFixture(replacement.label));
    await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
    await page.keyboard.press('Control+Shift+i');
    await page.getByRole('textbox', { name: 'Find Text', exact: true }).fill(replacement.query);
    await expect(page.locator('.match-count')).toHaveText(String((replacement.label.split(replacement.query).length - 1) * 2));
    await page.getByRole('textbox', { name: 'Replace With', exact: true }).fill(replacement.text);
    await page.getByRole('button', { name: 'Replace All', exact: true }).click();
    if (replacement.expected) {
      await expect(page.locator('a.docx-hyperlink')).toHaveText(replacement.expected);
      await expect(page.locator('a.docx-hyperlink')).toHaveAttribute('href', 'https://example.com/');
    } else await expect(page.locator('a.docx-hyperlink')).toHaveCount(0);
    await page.keyboard.press('Control+s');
    const saved = await page.evaluate(() => window.__nativeTest.saved);
    const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
    if (replacement.expected) {
      expect(xml).toContain('Original Tooltip');
      expect(xml).toMatch(/<w:b[\s\/>]/);
    }
  });
}

test('Replaces Body And Header Hyperlinks Together In One Undo Step', async ({ page }) => {
  const parts = unzipSync(new Uint8Array(hyperlinkFixture()));
  parts['[Content_Types].xml'] = strToU8(strFromU8(parts['[Content_Types].xml']).replace('</Types>', '<Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>'));
  parts['word/document.xml'] = strToU8(strFromU8(parts['word/document.xml']).replace('<w:sectPr>', '<w:sectPr><w:headerReference xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" w:type="default" r:id="rHeader"/>'));
  parts['word/_rels/document.xml.rels'] = strToU8(strFromU8(parts['word/_rels/document.xml.rels']).replace('</Relationships>', '<Relationship Id="rHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>'));
  parts['word/header1.xml'] = strToU8('<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:p><w:hyperlink r:id="rHeaderLink"><w:r><w:t>Hello World!</w:t></w:r></w:hyperlink></w:p></w:hdr>');
  parts['word/_rels/header1.xml.rels'] = strToU8('<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rHeaderLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.org/header" TargetMode="External"/></Relationships>');
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, Array.from(zipSync(parts)));
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await page.keyboard.press('Control+Shift+i');
  await page.getByRole('textbox', { name: 'Find Text', exact: true }).fill('Hello World!');
  await expect(page.locator('.match-count')).toHaveText('3');
  await page.getByRole('textbox', { name: 'Replace With', exact: true }).fill('Example');
  await page.getByRole('button', { name: 'Replace All', exact: true }).click();
  await expect(page.locator('.find-error')).toHaveCount(0);
  await page.keyboard.press('Control+s');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const replaced = unzipSync(new Uint8Array(saved!));
  for (const part of ['word/document.xml', 'word/header1.xml']) {
    const xml = strFromU8(replaced[part]);
    expect(xml).toContain('Example');
    expect(xml).not.toContain('Hello World!');
    expect(xml).toContain('w:hyperlink');
  }
  await page.getByRole('button', { name: 'Close Find', exact: true }).click();
  await page.keyboard.press('Control+z');
  await page.keyboard.press('Control+s');
  const undoBytes = await page.evaluate(() => window.__nativeTest.saved);
  const undone = unzipSync(new Uint8Array(undoBytes!));
  expect(strFromU8(undone['word/document.xml'])).toContain('Hello World!');
  expect(strFromU8(undone['word/header1.xml'])).toContain('Hello World!');
});

test('Learns A Custom Correction Through No Words Match And Remembers It', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('InsynDco works.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await expect(popup.getByRole('button', { name: 'No Words Match', exact: true })).toBeVisible();
  await popup.getByRole('button', { name: 'No Words Match', exact: true }).click();
  await expect(popup.getByRole('textbox', { name: 'Correct Word', exact: true })).toBeFocused();
  await expect(popup.getByRole('button', { name: 'Apply Correction', exact: true })).toBeDisabled();
  await popup.getByRole('textbox', { name: 'Correct Word', exact: true }).fill('InsynDoc');
  await expect(popup.getByText('Finding Suggestions...', { exact: true })).toHaveCount(0);
  await page.screenshot({ path: 'artifacts/ui/spelling-custom-correction.png' });
  await popup.getByRole('button', { name: 'Apply Correction', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toContainText('InsynDoc works.');
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('insyndoc.preferences.v1')!).spellingCorrections))
    .toEqual([{ source: 'insyndco', replacement: 'InsynDoc', count: 1 }]);
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').first()).toContainText('InsynDco works.');
  await page.reload();
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('InsynDco works.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  await expect(popup.locator('.spelling-suggestions button').first()).toHaveText('InsynDoc');
  await page.screenshot({ path: 'artifacts/ui/spelling-learned-suggestion.png' });
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Reset Learned Suggestions', exact: true }).click();
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('insyndoc.preferences.v1')!).spellingCorrections)).toEqual([]);
});

test('Cancels Manual Corrections Without Learning Or Changing The Document', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await popup.getByRole('button', { name: 'No Words Match', exact: true }).click();
  const input = popup.getByRole('textbox', { name: 'Correct Word', exact: true });
  for (const replacement of ['speling', 'two words']) {
    await input.fill(replacement);
    await expect(popup.getByRole('button', { name: 'Apply Correction', exact: true })).toBeDisabled();
  }
  await input.fill('spelling');
  await page.keyboard.press('Escape');
  await expect(popup).toHaveCount(0);
  await expect(page.locator('.docx-page-content').first()).toContainText('speling');
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('insyndoc.preferences.v1')!).spellingCorrections)).toEqual([]);
});

test('Checks Grammar Without The Offensive Language Warning', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('That is a fucking good idea. This is a example.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await expect(popup).toContainText('Grammar Suggestion');
  await expect(popup).not.toContainText('Try to avoid offensive language.');
  await expect(popup.getByRole('button', { name: 'an', exact: true })).toBeVisible();
  await page.screenshot({ path: 'artifacts/ui/spelling-grammar-without-language-warning.png' });
});

test('Underlines Misspelled Words And Applies An Undoable Suggestion', async ({ page }) => {
  const external: string[] = [];
  page.on('request', (request) => { if (!request.url().startsWith('http://127.0.0.1:1420/') && !request.url().startsWith('data:')) external.push(request.url()); });
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('This is a speling test.');
  const marks = page.locator('.spelling-mark');
  await expect(marks).toHaveCount(1);
  expect(await marks.first().evaluate((element) => getComputedStyle(element, '::after').maskImage)).toContain('spelling-underline.png');
  const bounds = await marks.first().boundingBox();
  await page.mouse.click(bounds!.x + bounds!.width / 2, bounds!.y + bounds!.height / 2);
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await expect(popup).toBeVisible();
  await expect(popup.getByRole('button', { name: 'Close Spelling Suggestions', exact: true })).not.toBeFocused();
  await page.screenshot({ path: 'artifacts/ui/spelling-suggestion.png' });
  await popup.getByRole('button', { name: 'spelling', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toContainText('This is a spelling test.');
  await expect(marks).toHaveCount(0);
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').first()).toContainText('speling');
  await expect(marks).toHaveCount(1);
  expect(external).toEqual([]);
});

test('Offers The Ranked Capitalized Correction First And Supports Undo', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('Teh cat is here.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  const first = popup.locator('.spelling-suggestions button').first();
  await expect(first).toHaveText('The');
  await first.click();
  await expect(page.locator('.docx-page-content').first()).toContainText('The cat is here.');
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('insyndoc.preferences.v1')!).spellingCorrections))
    .toEqual([{ source: 'teh', replacement: 'The', count: 1 }]);
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').first()).toContainText('Teh cat is here.');
});

test('Offers A Saved Personal Word For A Nearby Misspelling', async ({ page }) => {
  await page.evaluate(() => localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ spellingWords: ['InsynDoc'] })));
  await page.reload();
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('InsynDco works.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await expect(popup.locator('.spelling-suggestions button').first()).toHaveText('InsynDoc');
  await popup.getByRole('button', { name: 'InsynDoc', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toContainText('InsynDoc works.');
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
});

test('Keeps Spelling Marks Out Of The DOCX And Preserves Links When Correcting', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, hyperlinkFixture('speling'));
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(2);
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await popup.getByRole('button', { name: 'spelling', exact: true }).click();
  await expect(page.locator('a.docx-hyperlink')).toHaveText('spelling');
  await expect(page.locator('a.docx-hyperlink')).toHaveAttribute('href', 'https://example.com/');
  await page.keyboard.press('Control+s');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).not.toContain('spelling-mark');
  expect(xml).not.toContain('spelling-underline');
  expect(xml).toContain('Original Tooltip');
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
});

test('Can Disable Spelling And Persist A Personal Dictionary', async ({ page }) => {
  const dictionaryParts = unzipSync(new Uint8Array(hyperlinkFixture('speling')));
  dictionaryParts['word/document.xml'] = strToU8(strFromU8(dictionaryParts['word/document.xml']).replace('> speling<', '> and speling<'));
  const dictionaryBytes = Array.from(zipSync(dictionaryParts));
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, dictionaryBytes);
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(2);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('switch', { name: 'Check Spelling', exact: true }).uncheck();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('switch', { name: 'Check Spelling', exact: true }).check();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(2);
  await page.keyboard.press('Control+Alt+x');
  await page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true }).getByRole('button', { name: 'Add To Dictionary', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('insyndoc.preferences.v1')!).spellingWords)).toEqual(['speling']);
  await page.reload();
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, dictionaryBytes);
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await page.keyboard.press('Control+Alt+x');
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true })).toContainText('No Spelling Issues Found');
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Reset Personal Dictionary', exact: true }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(2);
  await page.keyboard.press('Control+Alt+x');
  await page.getByRole('button', { name: 'Ignore For This Session', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await expect(page.locator('.document-save-indicator')).toHaveText('Saved Locally');
});

test('Discards Stale Spelling Suggestions After Editing Or Opening Another Document', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await page.keyboard.press('Control+a');
  await page.keyboard.type('spelling');
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await page.keyboard.press('Control+s');
  await page.keyboard.press('Control+n');
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await page.getByRole('menuitem', { name: 'Tools', exact: true }).click();
  await page.getByRole('menuitem', { name: /^Check Spelling/ }).click();
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true })).toContainText('No Spelling Issues Found');
});

test('Continues Typing After Clicking A Spelling Underline', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  const mark = page.locator('.spelling-mark').first();
  await expect(mark).toBeVisible();
  const bounds = await mark.boundingBox();
  await page.mouse.click(bounds!.x + bounds!.width / 2, bounds!.y + bounds!.height / 2);
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true })).toBeVisible();
  await page.keyboard.press('End');
  await page.keyboard.type(' is a test.');
  await expect(page.locator('.docx-page-content').first()).toContainText('speling is a test.');
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true })).toHaveCount(0);
});

test('Keeps Editing Available When The Dictionary Fails And Supports Retry', async ({ page }) => {
  await page.route('**/*.dic', (route) => route.abort());
  await page.reload();
  await expect(page.locator('.spelling-error')).toContainText('Spelling Is Unavailable');
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
  await page.unroute('**/*.dic');
  await page.getByRole('button', { name: 'Retry Spelling', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await expect(page.locator('.spelling-error')).toHaveCount(0);
});

test('Keeps Spelling Suggestions Inside A Compact Dark Window', async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('This is wrng.');
  const mark = page.locator('.spelling-mark').first();
  await expect(mark).toBeVisible();
  await expect(page.locator('.docx-text-highlight-overlay').filter({ has: mark })).toHaveCSS('mix-blend-mode', 'multiply');
  await page.screenshot({ path: 'artifacts/ui/spelling-dark-underline.png' });
  const bounds = await mark.boundingBox();
  await page.mouse.click(bounds!.x + bounds!.width / 2, bounds!.y + bounds!.height / 2, { button: 'right' });
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions', exact: true });
  await expect(popup.getByRole('button', { name: 'wrong', exact: true })).toBeVisible();
  await page.screenshot({ path: 'artifacts/ui/spelling-compact-dark.png' });
  const popupBounds = await popup.boundingBox();
  expect(popupBounds!.x).toBeGreaterThanOrEqual(0);
  expect(popupBounds!.x + popupBounds!.width).toBeLessThanOrEqual(800);
  expect(popupBounds!.y + popupBounds!.height).toBeLessThanOrEqual(600);
});

test('Focuses Keyboard Spelling Suggestions And Accepts With Enter', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const suggestion = page.getByRole('dialog', { name: 'Spelling Suggestions' }).getByRole('button', { name: 'spelling', exact: true });
  await expect(suggestion).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('.docx-page-content').first()).toContainText('spelling');
});

test('Dismisses Pointer Spelling Suggestions Before Closing Find', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+f');
  const mark = await page.locator('.spelling-mark').first().boundingBox();
  await page.mouse.click(mark!.x + mark!.width / 2, mark!.y + mark!.height / 2);
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions' })).toHaveCount(0);
  await expect(page.getByRole('textbox', { name: 'Find Text', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('textbox', { name: 'Find Text', exact: true })).toHaveCount(0);
});

test('Does Not Reopen A Dismissed Pending Spelling Review', async ({ page }) => {
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  await page.route('**/*.dic', async (route) => { await held; await route.continue(); });
  await page.reload();
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions' });
  await expect(popup).toContainText('Checking Spelling');
  await page.keyboard.press('Escape');
  await expect(popup).toHaveCount(0);
  release();
  // A Later Explicit Review Waits For The Dictionary And Proves The Worker Finished.
  await page.waitForTimeout(1500);
  await expect(popup).toHaveCount(0);
  await page.keyboard.press('Control+Alt+x');
  await expect(popup).toContainText('No Spelling Issues Found');
});

test('Reports The Spelling Issue Limit', async ({ page }) => {
  const parts = unzipSync(new Uint8Array(fixture()));
  parts['word/document.xml'] = strToU8(strFromU8(parts['word/document.xml']).replace('Original Mixed CASE Content', 'speling '.repeat(1001)));
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, Array.from(zipSync(parts)));
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await page.keyboard.press('Control+Alt+x');
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions' })).toContainText('First 1,000 Spelling Issues');
});

interface SpellingWorkerControl {
  holdChecks: boolean;
  holdSuggestions: boolean;
  checks: number[];
  pending: Array<() => void>;
  release: () => void;
}

declare global {
  interface Window { __spellingTest: SpellingWorkerControl; }
}

async function controlSpellingWorker(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const control: SpellingWorkerControl = { holdChecks: false, holdSuggestions: false, checks: [], pending: [], release() {
      this.holdChecks = false;
      this.holdSuggestions = false;
      this.pending.splice(0).forEach((deliver) => deliver());
    } };
    window.__spellingTest = control;
    const NativeWorker = window.Worker;
    window.Worker = class extends NativeWorker {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        if (!String(url).includes('spelling.worker')) return;
        const post = this.postMessage.bind(this);
        this.postMessage = (message: unknown) => {
          const request = message as { type?: string; id?: number };
          if (request.type === 'check') control.checks.push(request.id!);
          post(message);
        };
        let delivering = false;
        this.addEventListener('message', (event: MessageEvent<{ type: string }>) => {
          if (delivering || !(event.data.type === 'checked' && control.holdChecks || event.data.type === 'suggested' && control.holdSuggestions)) return;
          event.stopImmediatePropagation();
          control.pending.push(() => {
            delivering = true;
            this.dispatchEvent(new MessageEvent('message', { data: event.data }));
            delivering = false;
          });
        });
      }
    };
  });
  await page.reload();
  await expect(page.getByRole('button', { name: 'Download Document', exact: true })).toBeEnabled();
  await page.keyboard.press('Control+Alt+x');
  await expect(page.getByRole('dialog', { name: 'Spelling Suggestions' })).toContainText('No Spelling Issues Found');
  await page.keyboard.press('Escape');
}

test('Does Not Steal Focus When Delayed Spelling Suggestions Arrive', async ({ page }) => {
  await controlSpellingWorker(page);
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.evaluate(() => { window.__spellingTest.holdSuggestions = true; });
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions' });
  await expect(popup.getByRole('button', { name: 'Close Spelling Suggestions' })).toBeFocused();
  await expect.poll(() => page.evaluate(() => window.__spellingTest.pending.length)).toBe(1);
  await page.keyboard.press('Tab');
  await expect(popup.getByRole('button', { name: 'No Words Match', exact: true })).toBeFocused();
  await page.keyboard.press('Tab');
  const ignore = popup.getByRole('button', { name: 'Ignore For This Session' });
  await expect(ignore).toBeFocused();
  await page.evaluate(() => window.__spellingTest.release());
  await expect(popup.getByRole('button', { name: 'spelling', exact: true })).toBeVisible();
  await expect(ignore).toBeFocused();
});

test('Keeps Manual Correction Focus When Delayed Suggestions Arrive', async ({ page }) => {
  await controlSpellingWorker(page);
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.evaluate(() => { window.__spellingTest.holdSuggestions = true; });
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions' });
  await expect.poll(() => page.evaluate(() => window.__spellingTest.pending.length)).toBe(1);
  await popup.getByRole('button', { name: 'No Words Match', exact: true }).click();
  const input = popup.getByRole('textbox', { name: 'Correct Word', exact: true });
  await input.fill('spelling');
  await page.evaluate(() => window.__spellingTest.release());
  await expect(popup.getByRole('button', { name: 'spelling', exact: true })).toBeVisible();
  await expect(input).toBeFocused();
  await expect(input).toHaveValue('spelling');
  await page.keyboard.press('Enter');
  await expect(page.locator('.docx-page-content').first()).toContainText('spelling');
});

test('Debounces New Edits After A Queued Slow Spelling Check', async ({ page }) => {
  await controlSpellingWorker(page);
  const initialChecks = await page.evaluate(() => window.__spellingTest.checks.length);
  await page.evaluate(() => { window.__spellingTest.holdChecks = true; });
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('speling');
  await expect.poll(() => page.evaluate(() => window.__spellingTest.pending.length)).toBe(1);
  await page.keyboard.type('x');
  await page.waitForTimeout(550);
  await page.keyboard.type('y');
  await page.evaluate(() => window.__spellingTest.release());
  await page.waitForTimeout(200);
  expect(await page.evaluate(() => window.__spellingTest.checks.length)).toBe(initialChecks + 1);
  await expect.poll(() => page.evaluate(() => window.__spellingTest.checks.length)).toBe(initialChecks + 2);
  await page.waitForTimeout(600);
  expect(await page.evaluate(() => window.__spellingTest.checks.length)).toBe(initialChecks + 2);
});

test('Checks Body And Table Spelling Without Offering Unsupported Textbox Corrections', async ({ page }) => {
  const parts = unzipSync(new Uint8Array(fixture()));
  const content = '<w:p><w:r><w:t>speling</w:t></w:r></w:p>';
  const body = content + '<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w="5000"/></w:tblGrid><w:tr><w:tc><w:tcPr><w:tcW w:w="5000" w:type="dxa"/></w:tcPr>' + content + '</w:tc></w:tr></w:tbl>';
  const frame = '<w:p><w:r><w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml" id="TextBox1" style="width:200pt;height:60pt" type="#_x0000_t202"><v:textbox><w:txbxContent>' + content + '</w:txbxContent></v:textbox></v:shape></w:pict></w:r></w:p>';
  parts['word/document.xml'] = strToU8(strFromU8(parts['word/document.xml']).replace('<w:p><w:r><w:t>Original Mixed CASE Content</w:t></w:r></w:p>', body + frame));
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, Array.from(zipSync(parts)));
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toContainText('speling');
  await expect(page.locator('.spelling-mark')).toHaveCount(2);
  await page.keyboard.press('Control+s');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  expect(strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml'])).toContain('txbxContent');
});

test('Uses Sentence Context To Suggest Be And Keeps Valid Me Unmarked', async ({ page }) => {
  const external: string[] = [];
  page.on('request', (request) => { if (!request.url().startsWith('http://127.0.0.1:1420/')) external.push(request.url()); });
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('I want to me there.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions' });
  await expect(popup).toContainText('Grammar Suggestion');
  await expect(popup.getByRole('button', { name: 'Add To Dictionary', exact: true })).toHaveCount(0);
  await expect(popup.getByRole('button', { name: 'be', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('.docx-page-content').first()).toContainText('I want to be there.');
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await page.keyboard.press('Control+z');
  await expect(page.locator('.docx-page-content').first()).toContainText('I want to me there.');
  await page.keyboard.press('Control+a');
  await page.keyboard.type('He gave it to me. Please be there for me.');
  await page.keyboard.press('Control+Alt+x');
  await expect(popup).toContainText('No Spelling Issues Found');
  expect(external).toEqual([]);
});

test('Corrects A Contextual Phrase And Ignores Only Its Sentence', async ({ page }) => {
  await page.locator('.docx-page-content').first().click({ position: { x: 30, y: 12 } });
  await page.keyboard.type('I could of helped.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  const popup = page.getByRole('dialog', { name: 'Spelling Suggestions' });
  await popup.getByRole('button', { name: 'could have', exact: true }).click();
  await expect(page.locator('.docx-page-content').first()).toContainText('I could have helped.');
  await page.keyboard.press('Control+a');
  await page.keyboard.type('I want to me there.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
  await page.keyboard.press('Control+Alt+x');
  await popup.getByRole('button', { name: 'Ignore For This Session', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(0);
  await page.keyboard.press('Control+a');
  await page.keyboard.type('It should me fine.');
  await expect(page.locator('.spelling-mark')).toHaveCount(1);
});

test('Preserves Hyperlinks When Applying A Sentence Context Suggestion', async ({ page }) => {
  await page.evaluate((bytes) => { window.__nativeTest.openBytes = bytes; }, hyperlinkFixture('I want to me there.'));
  await page.getByRole('button', { name: 'Import DOCX', exact: true }).click();
  await expect(page.locator('.spelling-mark')).toHaveCount(2);
  await page.keyboard.press('Control+Alt+x');
  await page.getByRole('dialog', { name: 'Spelling Suggestions' }).getByRole('button', { name: 'be', exact: true }).click();
  await expect(page.locator('a.docx-hyperlink')).toHaveText('I want to be there.');
  await expect(page.locator('a.docx-hyperlink')).toHaveAttribute('href', 'https://example.com/');
  await page.keyboard.press('Control+s');
  const saved = await page.evaluate(() => window.__nativeTest.saved);
  const xml = strFromU8(unzipSync(new Uint8Array(saved!))['word/document.xml']);
  expect(xml).toContain('Original Tooltip');
  expect(xml).toContain('w:hyperlink');
});
