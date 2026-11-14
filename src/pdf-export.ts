import { exportHtmlDocument } from './document-export';

export async function preparePdfDocument(bytes: Uint8Array, title: string): Promise<() => void> {
  if (document.getElementById('pdf-export-root')) throw new Error('A PDF download is already in progress.');
  const parsed = new DOMParser().parseFromString(await exportHtmlDocument(bytes, title), 'text/html');
  const root = document.createElement('div');
  root.id = 'pdf-export-root';
  for (const style of parsed.head.querySelectorAll('style')) root.appendChild(document.importNode(style, true));
  for (const child of parsed.body.childNodes) root.appendChild(document.importNode(child, true));
  document.body.appendChild(root);
  try {
    let timeout: ReturnType<typeof setTimeout> | undefined;
    try {
      await Promise.race([
        Promise.all([document.fonts.ready, ...[...root.querySelectorAll('img')].map((image) => image.decode())]),
        new Promise<never>((_, reject) => { timeout = setTimeout(() => reject(new Error('Document resources took too long to prepare for PDF.')), 15000); })
      ]);
    } finally { clearTimeout(timeout); }
    await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
    return () => root.remove();
  } catch (error) {
    root.remove();
    throw error;
  }
}
