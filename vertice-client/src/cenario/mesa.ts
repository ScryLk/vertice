import { Application, Container, Graphics } from 'pixi.js';

/**
 * A mesa: pedra escura e leylines convergindo.
 *
 * É cenário, não interface. As fichas continuam em React por cima — a divisão
 * que a `arquitetura.md` §1 prescreve ("React só pro chrome, Pixi pro
 * cenário"). Aqui só existe o que não precisa de texto.
 *
 * **Nenhum asset.** Tudo é geometria gerada, pela mesma razão que a carta é
 * ficha e não ilustração: 300 desenhos é impossível sozinho, e um cenário
 * procedural acompanha a direção iconográfica em vez de brigar com ela.
 *
 * A semente é fixa de propósito. O cenário pode variar entre partidas, mas
 * não entre recarregamentos — cena que muda sozinha a cada F5 parece defeito.
 */

const SEMENTE = 0x5e_ed_1a_7a;

/** Gerador determinístico local. Não usa `Math.random` pelo mesmo motivo. */
function sortear(estado: { valor: number }): number {
  estado.valor = (estado.valor * 1_664_525 + 1_013_904_223) >>> 0;
  return estado.valor / 0x1_0000_0000;
}

function interpolar(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

interface Leyline {
  angulo: number;
  desvio: number;
  espessura: number;
  atraso: number;
  brilho: number;
}

export interface Cenario {
  /** Chamar quando o componente desmonta. */
  destruir: () => void;
}

/** Quanto tempo a mesa leva para "abrir" quando o duelo começa. */
const DURACAO_DA_ENTRADA = 1.15;

export function montarMesa(app: Application): Cenario {
  const palco = new Container();
  const leylines = new Graphics();
  const nucleo = new Graphics();
  palco.addChild(leylines, nucleo);
  app.stage.addChild(palco);

  const estado = { valor: SEMENTE };
  const linhas: Leyline[] = Array.from({ length: 22 }, () => ({
    angulo: sortear(estado) * Math.PI * 2,
    desvio: (sortear(estado) - 0.5) * 1.1,
    espessura: 0.5 + sortear(estado) * 1.4,
    atraso: sortear(estado) * 0.4,
    brilho: 0.15 + sortear(estado) * 0.35,
  }));

  let decorrido = 0;

  const desenhar = () => {
    const { width, height } = app.screen;
    if (width === 0 || height === 0) return;

    const centroX = width / 2;
    // Acima do primeiro painel: o núcleo precisa ficar visível, não atrás de
    // uma caixa. É onde o olho procura o ponto de convergência.
    const centroY = height * 0.2;
    const alcance = Math.hypot(width, height) * 0.62;
    const entrada = Math.min(1, decorrido / DURACAO_DA_ENTRADA);

    // As leylines chegam do fora para o centro, cada uma no seu tempo.
    leylines.clear();
    for (const linha of linhas) {
      const t = Math.max(0, Math.min(1, (entrada - linha.atraso) / (1 - linha.atraso)));
      if (t <= 0) continue;

      const x0 = centroX + Math.cos(linha.angulo) * alcance;
      const y0 = centroY + Math.sin(linha.angulo) * alcance;
      const x1 = interpolar(x0, centroX, t);
      const y1 = interpolar(y0, centroY, t);

      const meioX = (x0 + x1) / 2 + Math.cos(linha.angulo + Math.PI / 2) * linha.desvio * 120 * (1 - t);
      const meioY = (y0 + y1) / 2 + Math.sin(linha.angulo + Math.PI / 2) * linha.desvio * 120 * (1 - t);

      leylines
        .moveTo(x0, y0)
        .quadraticCurveTo(meioX, meioY, x1, y1)
        .stroke({ width: linha.espessura, color: 0x6f8fa8, alpha: linha.brilho * t });
    }

    // O núcleo pulsa devagar depois que a mesa abre.
    const pulso = 1 + Math.sin(decorrido * 1.4) * 0.12;
    const raioNucleo = (10 + 26 * entrada) * pulso;

    nucleo.clear();
    nucleo.circle(centroX, centroY, raioNucleo).fill({ color: 0x9dc0d8, alpha: 0.05 * entrada });
    nucleo.circle(centroX, centroY, raioNucleo * 0.45).fill({ color: 0xc6dcea, alpha: 0.11 * entrada });
    nucleo.circle(centroX, centroY, 2.4).fill({ color: 0xe4f0f8, alpha: 0.5 * entrada });
  };

  const tick = () => {
    decorrido += app.ticker.deltaMS / 1000;
    desenhar();
  };

  app.ticker.add(tick);
  desenhar();

  return {
    destruir: () => {
      app.ticker.remove(tick);
      palco.destroy({ children: true });
    },
  };
}
