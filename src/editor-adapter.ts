import { createDocumentRefresh, type DocxEditorInstance } from '@docx-editor.dev/core/editor';
import { createBlankDocument } from './blank-document';
import type { Editor } from '@docx-editor.dev/core/contracts/editor';
import { localFonts } from './document-fonts';
import type { DocumentAdapter } from './document-session';
import { synchronizeImageExtents } from './image-export';

export { localFonts };

interface DocumentContentNode {
  readonly kind: string;
  readonly namespaceUri?: string;
  readonly localName?: string;
  readonly value?: string;
  readonly children?: readonly DocumentContentNode[];
}

function isEmptyStory(root: DocumentContentNode): boolean {
  const pending = [root];
  while (pending.length) {
    const node = pending.pop()!;
    if (node.kind === 'textValue') {
      if (node.value?.trim()) return false;
      continue;
    }
    if (node.namespaceUri !== 'http://schemas.openxmlformats.org/wordprocessingml/2006/main') return false;
    if (['pPr', 'rPr', 'sectPr'].includes(node.localName ?? '')) continue;
    if (!['document', 'body', 'p', 'r', 't', 'hdr', 'ftr', 'br', 'tab', 'cr'].includes(node.localName ?? '')) return false;
    pending.push(...node.children ?? []);
  }
  return true;
}

async function waitForDocument(editor: Editor): Promise<void> {
  const deadline = performance.now() + 30_000;
  while (true) {
    const snapshot = editor.snapshot();
    if (snapshot.parseError || snapshot.openFailure) {
      throw new Error('The document could not be opened. It may be corrupt, encrypted, or unsupported.');
    }
    if (!snapshot.isLoading && !snapshot.isOpening) return;
    if (performance.now() > deadline) throw new Error('Opening the document took too long. Try a smaller document.');
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  }
}

export function createEditorAdapter(editor: Editor): DocumentAdapter {
  // React's onReady Supplies The Mounted Engine Instance Behind The Editor Contract.
  const refresh = createDocumentRefresh(editor as DocxEditorInstance);
  return {
    async preparePdf(title) {
      await waitForDocument(editor);
      const bytes = new Uint8Array(await editor.save());
      const { preparePdfDocument } = await import('./pdf-export');
      return preparePdfDocument(bytes, title);
    },
    isEmpty() {
      const snapshot = editor.snapshot();
      if (snapshot.isLoading || snapshot.isOpening || snapshot.parseError || snapshot.openFailure || refresh.recoveryBytes()) return false;
      const parts = (editor as DocxEditorInstance).surface?.session.storyParts();
      return Boolean(parts?.length && parts.every((part) => isEmptyStory(part.root)));
    },
    async validate(bytes) {
      // Full Engine Validation Belongs To Load's Transaction, Before File Identity Changes.
      if (bytes.byteLength === 0) throw new Error('The document could not be opened. The file is empty.');
    },
    async load(bytes) {
      if (refresh.snapshot().recoveryAvailable && !await refresh.recover()) {
        throw new Error('The previous document could not be restored. Save a copy before reopening InsynDoc.');
      }
      const submission = await refresh.capture();
      try {
        const result = await refresh.applyUpdate({ submission, sequence: 0, bytes: bytes ?? createBlankDocument(), changes: [] });
        if (!result.ok && result.code === 'recovery-failed') {
          throw new Error('The previous document could not be restored. Save a copy before reopening InsynDoc.');
        }
        if (!result.ok) throw new Error('The document could not be opened. It may be corrupt, encrypted, or unsupported.');
      } finally {
        refresh.finish(submission);
      }
    },
    async save() {
      const recovery = refresh.recoveryBytes();
      if (recovery) return new Uint8Array(recovery);
      await waitForDocument(editor);
      return synchronizeImageExtents(new Uint8Array(await editor.save()));
    }
  };
}
