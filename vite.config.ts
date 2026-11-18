import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  worker: { format: 'es' },
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
  build: {
    target: 'es2022',
    rollupOptions: {
      output: {
        onlyExplicitManualChunks: true,
        manualChunks(id: string): string | undefined {
          if (!id.includes('node_modules')) return undefined;
          if (/\/node_modules\/(react|react-dom|scheduler)\//.test(id)) return 'react';
          // Keep the editor stylesheet in the entry's CSS cascade.
          if (id.includes('@docx-editor.dev') && !id.endsWith('.css')) return 'docx-editor';
          if (id.includes('harper.js') || id.includes('nspell') || id.includes('dictionary-en')) return 'spelling';
          if (id.includes('fflate')) return 'fflate';
          return undefined;
        }
      }
    }
  }
});
