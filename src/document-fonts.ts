import { packagedFonts } from '@docx-editor.dev/fonts';
import { GOOGLE_FONT_CATALOG, googleFonts } from '@docx-editor.dev/fonts/google';
import { composeFontOrigins, defineFontResolver } from '@docx-editor.dev/core/editor';
import families from './document-font-families.json';

const bundledUrls = new Map(GOOGLE_FONT_CATALOG.filter((face) => families.includes(face.family)).map((face) => [face.url, `/document-fonts/${new URL(face.url).pathname.split('/').pop()}`]));

const bundledFonts = googleFonts({
  allow: families,
  fetcher: (input, options) => {
    const requestedUrl = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    const localUrl = bundledUrls.get(requestedUrl);
    if (!localUrl) return Promise.reject(new Error('This Font Is Not Bundled With InsynDoc.'));
    return fetch(localUrl, options);
  }
});

const origins = [packagedFonts(), bundledFonts];
export const localFonts = defineFontResolver((request) => composeFontOrigins(origins, request));
