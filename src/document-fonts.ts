import { packagedFonts } from '@docx-editor.dev/fonts';
import { GOOGLE_FONT_CATALOG, googleFonts } from '@docx-editor.dev/fonts/google';
import { composeFontOrigins, defineFontResolver } from '@docx-editor.dev/core/editor';
import { isTauri } from '@tauri-apps/api/core';
import { invoke } from '@tauri-apps/api/core';
import { readPreferences } from './preferences';
import families from './document-font-families.json';

interface FontSource {
  request: { family: string; weight: number; style: 'normal' | 'italic' };
  id: string;
  bytes: Uint8Array;
  hash: string;
  faceIndex: number;
}

let bundledResolver: ReturnType<typeof googleFonts> | null = null;

function bundledFontResolver(): ReturnType<typeof googleFonts> {
  if (bundledResolver) return bundledResolver;
  const bundledUrls = new Map(GOOGLE_FONT_CATALOG.filter((face) => families.includes(face.family)).map((face) => [face.url, `/document-fonts/${new URL(face.url).pathname.split('/').pop()}`]));
  bundledResolver = googleFonts({
    allow: families,
    fetcher: (input, options) => {
      const requestedUrl = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      const localUrl = bundledUrls.get(requestedUrl);
      if (!localUrl) return Promise.reject(new Error('This font is not bundled with InsynDoc.'));
      return fetch(localUrl, options);
    }
  });
  return bundledResolver;
}

interface InstalledFontFace {
  family: string;
  weight: number;
  style: 'normal' | 'italic';
  path: string;
}

let catalogPromise: Promise<InstalledFontFace[]> | null = null;

function installedFontCatalog(): Promise<InstalledFontFace[]> {
  if (!isTauri()) return Promise.resolve([]);
  catalogPromise ??= invoke<InstalledFontFace[]>('installed_fonts').catch(() => []);
  return catalogPromise;
}

const fontBytes = new Map<string, Promise<Uint8Array>>();

function readFontBytes(path: string): Promise<Uint8Array> {
  let pending = fontBytes.get(path);
  if (!pending) {
    pending = invoke<ArrayBuffer>('read_font_file', { path }).then((buffer) => new Uint8Array(buffer)).catch(() => new Uint8Array());
    fontBytes.set(path, pending);
  }
  return pending;
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes as BufferSource);
  return `sha256:${Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, '0')).join('')}`;
}

const systemFonts = defineFontResolver(async (request) => {
  if (!readPreferences().localFonts) return undefined;
  const catalog = await installedFontCatalog();
  if (!catalog.length) return undefined;
  const wanted = new Map<string, string>();
  for (const family of request.families) wanted.set(family.toLowerCase(), family);
  wanted.set(request.defaultFamily.toLowerCase(), request.defaultFamily);
  const entries = catalog.filter((entry) => wanted.has(entry.family.toLowerCase()));
  if (!entries.length) return { supportedFamilies: [...new Set(catalog.map((entry) => entry.family))].slice(0, 400) };
  const seenFaces = new Set<string>();
  const sources: FontSource[] = [];
  for (const entry of entries) {
    const faceKey = `${entry.family.toLowerCase()} ${entry.weight} ${entry.style}`;
    if (seenFaces.has(faceKey)) continue;
    seenFaces.add(faceKey);
    const bytes = await readFontBytes(entry.path);
    if (!bytes.length) continue;
    sources.push({
      request: { family: entry.family, weight: entry.weight, style: entry.style },
      id: `local:${entry.path}:${entry.weight}:${entry.style}`,
      bytes,
      hash: await sha256(bytes),
      faceIndex: 0
    });
  }
  return {
    supportedFamilies: [...new Set(catalog.map((entry) => entry.family))].slice(0, 400),
    sources
  };
});

let origins: Parameters<typeof composeFontOrigins>[0] | null = null;

function fontOrigins(): Parameters<typeof composeFontOrigins>[0] {
  origins ??= [systemFonts, packagedFonts(), bundledFontResolver()];
  return origins;
}

export const localFonts = defineFontResolver((request) => composeFontOrigins(fontOrigins(), request));
