import { SuggestionKind, type Linter } from 'harper.js';
import { checkParagraphs, type SpellingIssue, type SpellingParagraph } from './spelling-engine';

export const WRITING_LINT_CONFIG = { SpellCheck: false, AvoidCurses: false };

export async function checkWriting(paragraphs: readonly SpellingParagraph[], correct: (word: string) => boolean, excluded: readonly string[], linter: Linter, ignoredGrammar: readonly string[] = []) {
  const issues: SpellingIssue[] = [];
  const ignored = new Set(ignoredGrammar);
  const words = new Map<string, boolean>();
  for (const paragraph of paragraphs) {
    const spelling = checkParagraphs([paragraph], (word) => {
      if (!words.has(word)) words.set(word, correct(word));
      return words.get(word)!;
    }, excluded);
    const candidates = [...spelling.issues];
    const protectedRanges = [...paragraph.text.matchAll(/(?:https?:\/\/|www\.)\S+|[\w.+-]+@[\w.-]+\.[a-z]{2,}/gi)]
      .map((match) => ({ start: match.index, end: match.index + match[0].length }));
    const addGrammar = (start: number, end: number, message: string, suggestions: string[]): void => {
      const word = paragraph.text.slice(start, end);
      if (!word || !suggestions.length || protectedRanges.some((range) => start < range.end && end > range.start)) return;
      if (candidates.some((issue) => start < issue.start + issue.length && end > issue.start)) return;
      const ignoreKey = JSON.stringify([paragraph.blockId, paragraph.text, start, message]);
      if (ignored.has(ignoreKey)) return;
      candidates.push({ blockId: paragraph.blockId, start, length: end - start, word, expectedText: word, kind: 'grammar', message, suggestions, ignoreKey });
    };
    const lints = await linter.lint(paragraph.text, { language: 'plaintext' });
    try {
      for (const lint of lints) {
        const span = lint.span();
        const start = span.start;
        const end = span.end;
        span.free();
        if (paragraph.text.slice(start, end) !== lint.get_problem_text()) continue;
        const source = paragraph.text.slice(start, end);
        const suggestions = lint.suggestions();
        try {
          const replacements = suggestions.map((suggestion) => suggestion.kind() === SuggestionKind.InsertAfter
            ? source + suggestion.get_replacement_text() : suggestion.get_replacement_text());
          addGrammar(start, end, lint.message(), [...new Set(replacements)].filter((replacement) => replacement !== source).slice(0, 5));
        } finally { suggestions.forEach((suggestion) => suggestion.free()); }
      }
    } finally { lints.forEach((lint) => lint.free()); }
    // Limit This Confusion Rule To Verb Slots, Leaving Object Pronouns Alone.
    const bePatterns = [
      /\b(?<prefix>(?:want|wants|wanted|need|needs|needed|hope|hopes|hoped|plan|plans|planned)[ \t]+to[ \t]+)(?<word>me)(?=[ \t]+(?:here|there|ready|able|happy|careful|honest|sure|safe|fine)\b)/gi,
      /\b(?<prefix>(?:I|you|he|she|we|they|it)[ \t]+(?:will|would|should|could|can|must|might|may)[ \t]+)(?<word>me)(?=[ \t]+(?:here|there|ready|able|happy|careful|honest|sure|safe|fine)\b)/gi
    ];
    for (const pattern of bePatterns) {
      for (const match of paragraph.text.matchAll(pattern)) {
        const start = match.index + match.groups!.prefix.length;
        const word = match.groups!.word;
        const replacement = word === 'ME' ? 'BE' : word === 'Me' ? 'Be' : 'be';
        addGrammar(start, start + word.length, 'This sentence needs the verb “be” here, rather than the pronoun “me”.', [replacement]);
      }
    }
    candidates.sort((left, right) => left.start - right.start);
    issues.push(...candidates.slice(0, 1001 - issues.length));
    if (issues.length > 1000 || spelling.truncated) return { issues: issues.slice(0, 1000), truncated: true };
  }
  return { issues, truncated: false };
}
