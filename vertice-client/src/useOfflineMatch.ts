import { useCallback, useEffect, useRef, useState } from 'react';

import type { ActionDto, PlayerViewDto } from './protocol';
import init, { Duel as WasmDuel } from './wasm/vertice_wasm';

export interface OfflineMatch {
  view: PlayerViewDto | null;
  erro: string | null;
  jogar: (acao: ActionDto) => void;
}

/**
 * Duelo offline: o motor compilado para WASM rodando na própria página.
 *
 * A view que sai daqui tem **a mesma forma** que a do servidor, então a tela
 * não sabe a diferença. O que muda é quem decide — e offline quem decide é o
 * navegador, sem autoridade nenhuma. É modo de campanha e de treino; partida
 * valendo continua sendo no servidor.
 *
 * O `init()` carrega o `.wasm` uma vez por página; chamar de novo é barato
 * (o wasm-bindgen guarda a instância).
 */
export function useOfflineMatch(nivel: 'calouro' | 'formando'): OfflineMatch {
  const [view, setView] = useState<PlayerViewDto | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const duelo = useRef<WasmDuel | null>(null);

  useEffect(() => {
    let vivo = true;

    (async () => {
      try {
        await init();
        if (!vivo) return;

        const criado = new WasmDuel(Math.floor(Math.random() * 1_000_000_000), nivel);
        duelo.current = criado;
        setView(JSON.parse(criado.view(0)) as PlayerViewDto);
      } catch (falha) {
        if (vivo) setErro(String(falha));
      }
    })();

    return () => {
      vivo = false;
      duelo.current?.free();
      duelo.current = null;
    };
  }, [nivel]);

  const jogar = useCallback((acao: ActionDto) => {
    const criado = duelo.current;
    if (!criado) return;

    try {
      criado.act(0, JSON.stringify(acao));
      setView(JSON.parse(criado.view(0)) as PlayerViewDto);
    } catch (falha) {
      setErro(String(falha));
    }
  }, []);

  return { view, erro, jogar };
}
