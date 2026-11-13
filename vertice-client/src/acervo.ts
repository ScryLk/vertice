import { useEffect, useState } from 'react';

import type { CardDefDto } from './protocol';

/**
 * O acervo indexado por `CardId`.
 *
 * A `PlayerView` diz *qual* carta está em cada zona — só o id —, e não o que
 * ela faz. Isso é de propósito: a view é estado de partida, e definição de
 * carta é dado público que o cliente já tem. Sem este índice, a mão do duelo
 * mostra nome e corpo, e o jogador escolhe entre "Conjurar X → alvo" sem saber
 * o que X faz.
 *
 * Vem do WASM, que já vai no bundle e é a mesma fonte que a tela do acervo
 * usa. Não passa pela rede: o cliente não deve depender do servidor para saber
 * o que uma carta faz, e uma partida offline não teria como perguntar.
 *
 * Chama-se `acervo` e não `catalogo` porque `Catalogo.tsx` já existe e o
 * sistema de arquivos do macOS não diferencia maiúscula de minúscula — dois
 * arquivos com o mesmo nome em caixas diferentes quebram o TypeScript.
 */
export type Acervo = Map<number, CardDefDto>;

let pendente: Promise<Acervo> | null = null;

/**
 * Carrega uma vez por página. Falha é devolvida, não engolida — mas quem chama
 * trata como "sem acervo", e a tela degrada para nome e corpo.
 */
export function carregarAcervo(): Promise<Acervo> {
  if (!pendente) {
    pendente = (async () => {
      const { default: init, catalogo } = await import('./wasm/vertice_wasm');
      await init();
      const cartas = JSON.parse(catalogo()) as CardDefDto[];
      return new Map(cartas.map((carta) => [carta.id, carta]));
    })().catch((erro) => {
      // Não guarda a falha para sempre: um tropeço no carregamento não pode
      // condenar a página inteira até o próximo F5.
      pendente = null;
      throw erro;
    });
  }
  return pendente;
}

/** `null` enquanto carrega, ou se o acervo não estiver disponível. */
export function useAcervo(): Acervo | null {
  const [acervo, setAcervo] = useState<Acervo | null>(null);

  useEffect(() => {
    let vivo = true;
    carregarAcervo()
      .then((carregado) => {
        if (vivo) setAcervo(carregado);
      })
      .catch(() => {
        // Sem acervo a tela continua jogável: ela só não sabe o texto das
        // cartas. Falhar em silêncio aqui é melhor que derrubar o duelo.
      });
    return () => {
      vivo = false;
    };
  }, []);

  return acervo;
}
