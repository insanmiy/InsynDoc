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
        manualChunks(id: string): string | undefined {
          if (!id.includes('node_modules')) return undefined;
          if (id.includes('@docx-editor.dev')) return 'docx-editor';
          if (id.includes('harper.js') || id.includes('nspell') || id.includes('dictionary-en')) return 'spelling';
          if (id.includes('fflate')) return 'fflate';
          return undefined;
        }
      }
    }
  }
});
