import { existsSync } from 'node:fs';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';

const env = { ...process.env };
const localCargo = path.resolve('.tools/cargo');
if (existsSync(path.join(localCargo, 'bin/cargo.exe'))) {
  env.CARGO_HOME = localCargo;
  env.RUSTUP_HOME = path.resolve('.tools/rustup');
  const pathKey = Object.keys(env).find((key) => key.toLowerCase() === 'path') ?? 'PATH';
  env[pathKey] = [path.join(localCargo, 'bin'), path.resolve('.tools/msvcrt/mingw64/bin'), env[pathKey]].join(path.delimiter);
}
const cargo = process.argv[2] === '--cargo';
if (process.platform === 'win32' && !cargo && process.argv[2] === 'build') {
  const preparation = spawnSync('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.resolve('scripts/prepare-build.ps1')], { stdio: 'inherit', env });
  if (preparation.error) { console.error(preparation.error.message); process.exit(1); }
  if (preparation.status !== 0) process.exit(preparation.status ?? 1);
}
const command = cargo ? 'cargo' : process.execPath;
const args = cargo ? process.argv.slice(3) : [path.resolve('node_modules/@tauri-apps/cli/tauri.js'), ...process.argv.slice(2)];
const child = spawn(command, args, { stdio: 'inherit', env });
child.on('error', (error) => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code) => { process.exitCode = code ?? 1; });
