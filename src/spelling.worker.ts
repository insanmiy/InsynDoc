import nspell from 'nspell';
import { LocalLinter, Dialect, createBinaryModuleFromUrl } from 'harper.js';
import { checkWriting, WRITING_LINT_CONFIG } from './grammar-engine';
import { suggestSpelling, type SpellingRequest, type SpellingResponse } from './spelling-engine';

let grammar: Promise<LocalLinter> | null = null;
function loadGrammar(): Promise<LocalLinter> {
  grammar ??= (async () => {
    const linter = new LocalLinter({ binary: createBinaryModuleFromUrl(new URL('../node_modules/harper.js/dist/harper_wasm_slim_bg.wasm', import.meta.url).href, 'slim'), dialect: Dialect.American });
    await linter.setLintConfig(WRITING_LINT_CONFIG);
    return linter;
  })();
  return grammar;
}

let dictionary: Promise<ReturnType<typeof nspell>> | null = null;

function loadDictionary(): Promise<ReturnType<typeof nspell>> {
  dictionary ??= Promise.all([
    fetch(new URL('../node_modules/dictionary-en/index.aff?no-inline', import.meta.url)),
    fetch(new URL('../node_modules/dictionary-en/index.dic', import.meta.url))
  ]).then(async ([affix, words]) => {
    if (!affix.ok || !words.ok) throw new Error('The Spelling Dictionary Could Not Be Loaded.');
    // Treat All Dictionary Words Equally When Generating Suggestions.
    const rules = (await affix.text()).replace(/^NOSUGGEST[^\r\n]*(?:\r?\n|$)/gm, '');
    return nspell(rules, await words.text());
  });
  return dictionary;
}

self.onmessage = async (event: MessageEvent<SpellingRequest>): Promise<void> => {
  const request = event.data;
  let response: SpellingResponse;
  try {
    const checker = await loadDictionary();
    if (request.type === 'check') {
      const linter = await loadGrammar();
      response = { type: 'checked', id: request.id, ...await checkWriting(request.paragraphs, (word) => checker.correct(word), request.excluded, linter, request.ignoredGrammar) };
    } else {
      response = { type: 'suggested', id: request.id, suggestions: suggestSpelling(request.word, checker, request.personalWords, request.corrections) };
    }
  } catch {
    response = { type: 'error', id: request.id };
  }
  self.postMessage(response);
};
