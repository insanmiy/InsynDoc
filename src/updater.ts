import { isTauri } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export interface PendingUpdate {
  update: Update;
  version: string;
  notes: string | null;
}

export async function findUpdate(): Promise<PendingUpdate | null> {
  if (!isTauri()) return null;
  try {
    const update = await check();
    if (!update) return null;
    return { update, version: update.version, notes: update.body ?? null };
  } catch {
    return null;
  }
}

export async function installUpdate(pending: PendingUpdate, onProgress?: (downloaded: number, total: number | undefined) => void): Promise<void> {
  let downloaded = 0;
  let total: number | undefined;
  await pending.update.downloadAndInstall((event) => {
    if (event.event === 'Started') total = event.data.contentLength;
    if (event.event === 'Progress') downloaded += event.data.chunkLength;
    if (event.event === 'Finished') downloaded = total ?? downloaded;
    onProgress?.(downloaded, total);
  });
  await relaunch();
}
