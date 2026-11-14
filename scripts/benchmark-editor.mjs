import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { chromium } from '@playwright/test';
import { preview } from 'vite';

const baselineDirectory = path.resolve(process.argv[2] ?? '.bench_workspace/page-count-baseline/dist');
const currentDirectory = path.resolve(process.argv[3] ?? 'dist');
const outputPath = path.resolve(process.argv[4] ?? '.bench_workspace/results/editor-paste-comparison.json');
const runs = Number(process.env.BENCHMARK_RUNS ?? 6);
if (!Number.isInteger(runs) || runs < 2) throw new Error('Benchmark runs must be an integer of at least two.');

const variants = [
  { name: 'baseline', directory: baselineDirectory, port: 1431 },
  { name: 'changed', directory: currentDirectory, port: 1432 }
];
const scenarios = ['paragraphs', 'long-paragraph', 'rich-html'];
const servers = [];
const results = [];
let browser;

function summarize(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return {
    median: sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2,
    p95: sorted[Math.ceil(sorted.length * 0.95) - 1]
  };
}

try {
  for (const variant of variants) {
    servers.push(await preview({
      configFile: false,
      build: { outDir: variant.directory },
      preview: { host: '127.0.0.1', port: variant.port, strictPort: true }
    }));
  }
  browser = await chromium.launch({ channel: 'msedge', headless: true });
  for (const scenario of scenarios) {
    for (let run = 0; run <= runs; run++) {
      const orderedVariants = run % 2 ? [...variants].reverse() : variants;
      for (const variant of orderedVariants) {
        const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
        try {
          await page.addInitScript(() => {
            localStorage.setItem('insyndoc.preferences.v1', JSON.stringify({ checkSpelling: false, updateNotifications: false }));
            window.isTauri = true;
            window.__TAURI_INTERNALS__ = {
              metadata: { currentWindow: { label: 'main' } },
              invoke: async (command) => {
                if (command === 'open_startup_document') return new Uint8Array([4, 0, 0, 0, 110, 117, 108, 108]).buffer;
                if (command === 'installed_fonts') return [];
                if (command === 'caps_lock_on') return false;
                if (command === 'document_statistics') return { words: 0, characters: 0, charactersWithoutSpaces: 0 };
                return null;
              }
            };
          });
          await page.goto(`http://127.0.0.1:${variant.port}`, { waitUntil: 'domcontentloaded' });
          const editor = page.locator('.docx-pages[contenteditable="true"]');
          await editor.waitFor();
          await editor.focus();
          const session = await page.context().newCDPSession(page);
          await session.send('Performance.enable');
          const before = await session.send('Performance.getMetrics');
          const paste = await editor.evaluate(async (element, name) => {
            const sentence = 'This paragraph contains ordinary document text to measure paste performance and continued editing responsiveness. ';
            const body = sentence.repeat(Math.ceil(300_000 / sentence.length)).slice(0, 300_000);
            const paragraphs = body.match(/.{1,600}/g);
            const text = (name === 'long-paragraph' ? body : paragraphs.join('\n')) + '\nPaste end marker';
            const clipboardData = new DataTransfer();
            clipboardData.setData('text/plain', text);
            if (name === 'rich-html') clipboardData.setData('text/html', [...paragraphs, 'Paste end marker'].map((paragraph) => `<p><b>${paragraph.slice(0,100)}</b>${paragraph.slice(100)}</p>`).join(''));
            const start = performance.now();
            element.dispatchEvent(new ClipboardEvent('paste', { clipboardData, bubbles: true, cancelable: true }));
            const handlerMs = performance.now() - start;
            await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
            return { characters: text.length, handlerMs, nextFramesMs: performance.now() - start };
          }, scenario);
          await page.waitForFunction(() => document.querySelector('.docx-pages')?.textContent.includes('Paste end marker'));
          await page.waitForTimeout(500);
          const typingMs = [];
          // Move into the large paragraph rather than typing in the small end marker.
          await page.keyboard.press('ArrowUp');
          await page.keyboard.press('End');
          for (let key = 0; key < 5; key++) {
            const revision = await editor.getAttribute('data-revision');
            const start = performance.now();
            await page.keyboard.insertText('x');
            await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
            typingMs.push(performance.now() - start);
            if (await editor.getAttribute('data-revision') === revision) throw new Error('The typing probe did not change the document.');
          }
          await page.waitForTimeout(500);
          await page.waitForFunction(() => {
            const count = Number(document.querySelector('.document-status button[aria-label="Go to page"]')?.textContent.match(/\d+/)?.[0]);
            return count > 1 && count === document.querySelectorAll('.docx-page').length;
          });
          const after = await session.send('Performance.getMetrics');
          const metric = (data, name) => data.metrics.find((entry) => entry.name === name)?.value ?? 0;
          const result = {
            variant: variant.name, scenario, run, ...paste, typingMs,
            taskCpuMs: (metric(after, 'TaskDuration') - metric(before, 'TaskDuration')) * 1000,
            pages: await page.locator('.docx-page').count()
          };
          if (run > 0) results.push(result);
          console.log(JSON.stringify({ variant: variant.name, scenario, run, pasteMs: paste.nextFramesMs, typingMedianMs: summarize(typingMs).median }));
        } finally {
          await page.close();
        }
      }
    }
  }
  const summary = scenarios.flatMap((scenario) => variants.map((variant) => {
    const samples = results.filter((result) => result.scenario === scenario && result.variant === variant.name);
    return {
      scenario, variant: variant.name,
      pasteHandlerMs: summarize(samples.map((sample) => sample.handlerMs)),
      pasteNextFramesMs: summarize(samples.map((sample) => sample.nextFramesMs)),
      typingNextFramesMs: summarize(samples.flatMap((sample) => sample.typingMs)),
      taskCpuMs: summarize(samples.map((sample) => sample.taskCpuMs))
    };
  }));
  await mkdir(path.dirname(outputPath), { recursive: true });
  await writeFile(outputPath, JSON.stringify({
    timestamp: new Date().toISOString(), runs, variants,
    scope: 'Edge production frontend with mocked desktop calls and spelling disabled. Synthetic clipboard events. Typing uses keyboard insertText. Frame timings are animation frame proxies, not verified screen presentation. CPU includes paste, typing, and two settling intervals. One warmup per variant and scenario is excluded.',
    summary, results
  }, null, 2));
  console.log(`Saved benchmark results to ${outputPath}`);
} finally {
  await browser?.close();
  for (const server of servers) await new Promise((resolve, reject) => server.httpServer.close((error) => error ? reject(error) : resolve()));
}
