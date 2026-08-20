import { defineConfig } from 'vite';
import solidExtension from 'vite-plugin-solid';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

export default defineConfig({
  plugins: [
    solidExtension(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      '@extensions': path.resolve(__dirname, 'extensions'),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: 'esnext',
    minify: 'esbuild',
    sourcemap: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
});