import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig(() => {
  const host = process.env.TAURI_DEV_HOST || '127.0.0.1';
  const port = Number(process.env.TAURI_DEV_PORT) || 5173;

  return {
    plugins: [vue()],
    server: {
      host,
      port,
      strictPort: true,
      hmr: {
        protocol: process.env.TAURI_DEV_PROTOCOL || 'ws',
        host,
        port,
      },
    },
    envPrefix: ['VITE_', 'TAURI_'],
    build: {
      target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
      minify: process.env.TAURI_ENV_DEBUG ? false : 'esbuild',
      sourcemap: !!process.env.TAURI_ENV_DEBUG,
    },
  };
});