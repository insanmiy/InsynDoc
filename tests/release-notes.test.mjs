import assert from 'node:assert/strict';
import { test } from 'node:test';
import { releaseNotesFor } from '../scripts/release-notes.mjs';

const changelog = '# Changelog\n\n## [Unreleased]\n\n- Future changes.\n\n## [1.2.3] - 2026-11-13\n\n### Fixed\n\n- Corrected spelling suggestions.\n\n## [1.2.2]\n\n- Older changes.\n';

test('Extracts only the tagged version and preserves Markdown', () => {
  assert.equal(releaseNotesFor(changelog, 'v1.2.3', '1.2.3'), '### Fixed\n\n- Corrected spelling suggestions.\n');
});

test('Handles Windows line endings and a final section without a date', () => {
  assert.equal(releaseNotesFor(changelog.replaceAll('\n', '\r\n'), 'v1.2.2', '1.2.2'), '- Older changes.\n');
});

test('Rejects mismatched tags', () => {
  assert.throws(() => releaseNotesFor(changelog, 'v1.2.4', '1.2.3'), /tag must match/);
});

test('Requires versioned notes instead of publishing Unreleased', () => {
  assert.throws(() => releaseNotesFor(changelog, 'v1.2.5', '1.2.5'), /exactly one/);
});

test('Rejects duplicate version sections', () => {
  assert.throws(() => releaseNotesFor(`${changelog}\n## [1.2.3]\n\n- Duplicate.`, 'v1.2.3', '1.2.3'), /exactly one/);
});

test('Rejects a version with no change entries', () => {
  assert.throws(() => releaseNotesFor('## [1.2.3]\n\n### Fixed\n', 'v1.2.3', '1.2.3'), /Add release entries/);
});
