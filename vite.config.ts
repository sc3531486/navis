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
    },
  },
  // Tauri 开发环境配置
  server: {
    port: 1420,
    strictPort: true,
  },
  // Tauri 构建配置
  build: {
    target: 'esnext',
    minify: 'esbuild',
    sourcemap: true,
  },
  // 防止 Vite 遮蔽 Rust 错误
  envPrefix: ['VITE_', 'TAURI_'],
});
