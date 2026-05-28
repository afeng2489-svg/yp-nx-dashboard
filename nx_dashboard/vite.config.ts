import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 1420,
    /** 专用端口，避免与默认 Vite 5173（其他项目）冲突；须与 tauri.conf.json devUrl 一致 */
    strictPort: true,
    // 忽略数据库/构建/状态文件变化，防止运行时写入触发 HMR 无限重载
    watch: {
      ignored: [
        '**/node_modules/**',
        '**/.git/**',
        '**/nexus.db*',
        '**/nexus_memory.db*',
        '**/.omc/**',
        '**/target/**',
        '**/dist/**',
        '**/binaries/**',
      ],
    },
    proxy: {
      '/health': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true,
        changeOrigin: true,
      },
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    exclude: ['e2e/**', 'node_modules/**'],
  },
});
