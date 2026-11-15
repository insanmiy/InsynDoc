import nspell from 'nspell';
import type { Linter } from 'harper.js';
import { checkParagraphs, createSpellingFallback, suggestSpelling, type SpellingRequest, type SpellingResponse } from './spelling-engine';

let grammar: Promise<Linter> | null = null;
function loadGrammar(): Promise<Linter> {
  grammar ??= (async () => {
    const [{ LocalLinter, Dialect, createBinaryModuleFromUrl }, { WRITING_LINT_CONFIG }] = await Promise.all([import('harper.js'), import('./grammar-engine')]);
    const linter = new LocalLinter({ binary: createBinaryModuleFromUrl(new URL('../node_modules/harper.js/dist/harper_wasm_slim_bg.wasm', import.meta.url).href, 'slim'), dialect: Dialect.American });
    await linter.setLintConfig(WRITING_LINT_CONFIG);
    return linter;
  })();
  return grammar;
}

let dictionary: Promise<ReturnType<typeof nspell>> | null = null;
let fallback: ((word: string) => string[]) | undefined;
let fallbackDictionary: string | undefined;

function loadDictionary(): Promise<ReturnType<typeof nspell>> {
  dictionary ??= Promise.all([
    fetch(new URL('../node_modules/dictionary-en/index.aff?no-inline', import.meta.url)),
    fetch(new URL('../node_modules/dictionary-en/index.dic', import.meta.url))
  ]).then(async ([affix, words]) => {
    if (!affix.ok || !words.ok) throw new Error('The spelling dictionary could not be loaded.');
    // Treat All Dictionary Words Equally When Generating Suggestions.
    const rules = (await affix.text()).replace(/^NOSUGGEST[^\r\n]*(?:\r?\n|$)/gm, '');
    const dictionaryText = await words.text();
    fallbackDictionary = dictionaryText;
    return nspell(rules, dictionaryText);
  });
  return dictionary;
}

self.onmessage = async (event: MessageEvent<SpellingRequest>): Promise<void> => {
  const request = event.data;
  let response: SpellingResponse;
  try {
    const checker = await loadDictionary();
    if (request.type === 'check') {
      if (request.grammar) {
        const [{ checkWriting }, linter] = await Promise.all([import('./grammar-engine'), loadGrammar()]);
        response = { type: 'checked', id: request.id, ...await checkWriting(request.paragraphs, (word) => checker.correct(word), request.excluded, linter, request.ignoredGrammar) };
      } else {
        response = { type: 'checked', id: request.id, ...checkParagraphs(request.paragraphs, (word) => checker.correct(word), request.excluded) };
      }
    } else {
      fallback ??= createSpellingFallback(fallbackDictionary ?? '');
      fallbackDictionary = undefined;
      response = { type: 'suggested', id: request.id, suggestions: suggestSpelling(request.word, {
        correct: (word) => checker.correct(word), suggest: (word) => checker.suggest(word), fallback
      }, request.personalWords, request.corrections) };
    }
  } catch {
    response = { type: 'error', id: request.id };
  }
  self.postMessage(response);
};
