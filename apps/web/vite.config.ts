import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules/echarts') || id.includes('node_modules/zrender')) {
            return 'vendor-echarts';
          }
          if (id.includes('node_modules/@xterm')) {
            return 'vendor-xterm';
          }
          if (id.includes('node_modules/lucide-svelte')) {
            return 'vendor-icons';
          }
          return undefined;
        },
      },
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:9480',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://127.0.0.1:9480',
        ws: true,
      },
    },
  },
});
