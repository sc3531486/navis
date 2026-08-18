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
      // 扩展前端代码别名
      '@navis-code': path.resolve(__dirname, 'extensions/navis-code/ExtensionUI/src'),
      '@agent-core': path.resolve(__dirname, 'extensions/navis-code/navis-agent-core/ExtensionUI/src'),
      '@ai-platform': path.resolve(__dirname, 'extensions/navis-code/navis-ai-platform/ExtensionUI/src'),
      '@session': path.resolve(__dirname, 'extensions/navis-code/navis-session/ExtensionUI/src'),
      '@editor-ext': path.resolve(__dirname, 'extensions/navis-code/navis-editor/ExtensionUI/src'),
      '@terminal-ext': path.resolve(__dirname, 'extensions/navis-code/navis-terminal/ExtensionUI/src'),
      '@settings-ext': path.resolve(__dirname, 'extensions/navis-code/navis-settings/ExtensionUI/src'),
      '@project-ext': path.resolve(__dirname, 'extensions/navis-code/navis-project/ExtensionUI/src'),
      '@task-ext': path.resolve(__dirname, 'extensions/navis-code/navis-task/ExtensionUI/src'),
      '@knowledge': path.resolve(__dirname, 'extensions/navis-code/navis-knowledge/ExtensionUI/src'),
      '@memory': path.resolve(__dirname, 'extensions/navis-code/navis-memory/ExtensionUI/src'),
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
