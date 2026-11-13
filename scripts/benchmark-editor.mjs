import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { chromium } from '@playwright/test';
import { preview } from 'vite';

const baselineDirectory = path.resolve(process.argv[2] ?? '.bench_workspace/page-count-baseline/dist');
const currentDirectory = path.resolve(process.argv[3] ?? 'dist');
const outputPath = path.resolve(process.argv[4] ?? '.bench_workspace/results/editor-paste-comparison.json');
const runs = Number(process.env.BENCHMARK_RUNS ?? 6);
if (!Number.isInteger(runs) || runs < 2) throw new Error('Benchmark Runs Must Be An Integer Of At Least Two.');

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
            const sentence = 'This Paragraph Contains Ordinary Document Text To Measure Paste Performance And Continued Editing Responsiveness. ';
            const body = sentence.repeat(Math.ceil(300_000 / sentence.length)).slice(0, 300_000);
            const paragraphs = body.match(/.{1,600}/g);
            const text = (name === 'long-paragraph' ? body : paragraphs.join('\n')) + '\nPaste End Marker';
            const clipboardData = new DataTransfer();
            clipboardData.setData('text/plain', text);
            if (name === 'rich-html') clipboardData.setData('text/html', [...paragraphs, 'Paste End Marker'].map((paragraph) => `<p><b>${paragraph.slice(0,100)}</b>${paragraph.slice(100)}</p>`).join(''));
            const start = performance.now();
            element.dispatchEvent(new ClipboardEvent('paste', { clipboardData, bubbles: true, cancelable: true }));
            const handlerMs = performance.now() - start;
            await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
            return { characters: text.length, handlerMs, nextFramesMs: performance.now() - start };
          }, scenario);
          await page.waitForFunction(() => document.querySelector('.docx-pages')?.textContent.includes('Paste End Marker'));
          await page.waitForTimeout(500);
          const typingMs = [];
          // Move Into The Large Paragraph Rather Than Typing In The Small End Marker.
          await page.keyboard.press('ArrowUp');
          await page.keyboard.press('End');
          for (let key = 0; key < 5; key++) {
            const revision = await editor.getAttribute('data-revision');
            const start = performance.now();
            await page.keyboard.insertText('x');
            await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
            typingMs.push(performance.now() - start);
            if (await editor.getAttribute('data-revision') === revision) throw new Error('The Typing Probe Did Not Change The Document.');
          }
          await page.waitForTimeout(500);
          await page.waitForFunction(() => {
            const count = Number(document.querySelector('.document-status button[aria-label="Go To Page"]')?.textContent.match(/\d+/)?.[0]);
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
    scope: 'Edge Production Frontend With Mocked Desktop Calls And Spelling Disabled. Synthetic Clipboard Events. Typing Uses Keyboard InsertText. Frame Timings Are Animation Frame Proxies, Not Verified Screen Presentation. CPU Includes Paste, Typing, And Two Settling Intervals. One Warmup Per Variant And Scenario Is Excluded.',
    summary, results
  }, null, 2));
  console.log(`Saved Benchmark Results To ${outputPath}`);
} finally {
  await browser?.close();
  for (const server of servers) await new Promise((resolve, reject) => server.httpServer.close((error) => error ? reject(error) : resolve()));
}
