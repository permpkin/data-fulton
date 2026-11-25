import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'node:path';

export default defineConfig({
  root: 'src/renderer',
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': resolve(import.meta.dirname, 'src/renderer/src'),
      '@shared': resolve(import.meta.dirname, 'src/shared'),
    },
  },
  clearScreen: false,
  server: {
    port: 5199,
    strictPort: true,
  },
  build: {
    outDir: '../../dist-renderer',
    emptyOutDir: true,
  },
});
