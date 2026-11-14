import type { DocxEditorInstance } from '@docx-editor.dev/core/editor';
import type { TextMatch } from '@docx-editor.dev/core/contracts/editor';
import type { TreeDocOp } from '@docx-editor.dev/core/store';

export function replaceDocumentMatches(editor: DocxEditorInstance, matches: readonly TextMatch[], replacement: string): string | null {
  const originalScope = editor.getActiveScope();
  const surface = editor.surface;
  if (!surface) return 'The editor is not ready.';
  const groups: Array<{ scope: ReturnType<typeof surface.storyScope>; ops: TreeDocOp[] }> = [];
  try {
    for (const match of matches) {
      editor.setActiveScope(match.scope ?? { kind: 'body' });
      if (!editor.can({ type: 'replaceMatch', match, text: replacement }).ok) {
        return 'This match could not be replaced. It may be in protected content.';
      }
      const scope = surface.storyScope();
      let group = groups.find((candidate) => JSON.stringify(candidate.scope) === JSON.stringify(scope));
      if (!group) { group = { scope, ops: [] }; groups.push(group); }
      // Insert Inside The Original Run Before Deleting Its Old Text To Preserve Hyperlinks.
      if (replacement) group.ops.push({ op: 'insertText', paragraphId: match.blockId, offset: match.start, text: replacement, bias: 'right' });
      group.ops.push({ op: 'deleteText', paragraphId: match.blockId, start: match.start + replacement.length, end: match.start + replacement.length + match.length });
    }
    const rejection = surface.commitReviewOps((historyToken) => {
      const result = surface.session.applyTreeOpsAtomic(groups, { historyToken });
      return { committed: result.committed, reason: result.reason };
    }, 'package-scoped');
    return rejection ? 'The replacement could not be completed. Your original text was kept.' : null;
  } finally { editor.setActiveScope(originalScope); }
}
