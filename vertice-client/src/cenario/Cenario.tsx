import { Application } from 'pixi.js';
import { useEffect, useRef } from 'react';

import { montarMesa } from './mesa';

/**
 * O cenário do duelo: uma camada Pixi atrás do React.
 *
 * A divisão é a da `arquitetura.md`: Pixi desenha o que não tem texto (pedra,
 * leyline, luz), React desenha o que tem (ficha, botão, número). Nenhum dos
 * dois tenta fazer o trabalho do outro — mover as fichas para dentro do Pixi
 * significaria reimplementar layout e tipografia que o navegador já faz bem.
 *
 * Se o WebGL não existir, o cenário simplesmente não aparece: a mesa continua
 * jogável, porque nada aqui carrega informação. É por isso que ele pode falhar
 * em silêncio.
 */
export function Cenario() {
  const hospedeiro = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const alvo = hospedeiro.current;
    if (!alvo) return;

    const app = new Application();
    let vivo = true;
    let cenario: { destruir: () => void } | null = null;

    (async () => {
      try {
        await app.init({
          background: '#0e1013',
          antialias: true,
          resizeTo: alvo,
          resolution: Math.min(window.devicePixelRatio || 1, 2),
          autoDensity: true,
        });

        if (!vivo) {
          app.destroy(true);
          return;
        }

        alvo.appendChild(app.canvas);
        cenario = montarMesa(app);
      } catch {
        // Sem WebGL não há cenário — e não há prejuízo: ele não carrega
        // informação nenhuma.
      }
    })();

    return () => {
      vivo = false;
      cenario?.destruir();
      if (app.renderer) app.destroy(true, { children: true });
    };
  }, []);

  return <div className="cenario" ref={hospedeiro} aria-hidden="true" />;
}
