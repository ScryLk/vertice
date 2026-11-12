import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// O proxy existe para o cliente não precisar de CORS no servidor: em
// desenvolvimento o navegador fala com o Vite, e o Vite fala com o Rust.
// Em produção os dois saem da mesma origem, e a configuração some.
export default defineConfig({
  plugins: [react()],
  // O mesmo arquivo serve o dev server e o Vitest: os dois passam pelo
  // plugin do React, e configurar duas vezes é uma chance a mais de divergir.
  test: {
    environment: 'jsdom',
  },
  server: {
    port: 5173,
    proxy: {
      '/matches': 'http://127.0.0.1:8080',
      '/ws': { target: 'ws://127.0.0.1:8080', ws: true },
    },
  },
});
