import { spawn } from 'node:child_process';

const jobs = [
  {
    name: 'typecheck',
    command: process.execPath,
    args: ['node_modules/typescript/bin/tsc', '--noEmit'],
  },
  {
    name: 'vite build',
    command: process.execPath,
    args: ['node_modules/vite/bin/vite.js', 'build'],
  },
];

const results = await Promise.all(jobs.map((job) => new Promise((resolve) => {
  const child = spawn(job.command, job.args, { stdio: 'inherit' });
  child.on('error', (error) => {
    console.error(`[${job.name}] Failed to start: ${error.message}`);
    resolve(1);
  });
  child.on('close', (code) => {
    if (code !== 0) {
      console.error(`[${job.name}] Exited with code ${code}`);
    }
    resolve(code ?? 1);
  });
})));

if (results.some((code) => code !== 0)) {
  process.exit(1);
}
