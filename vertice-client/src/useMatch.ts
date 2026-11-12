import { useCallback, useEffect, useRef, useState } from 'react';

import {
  PROTOCOL_VERSION,
  type ActionDto,
  type ClientMessage,
  type PlayerViewDto,
  type ServerMessage,
} from './protocol';

export type Estado = 'ocioso' | 'conectando' | 'conectado' | 'caido' | 'encerrado';

export interface Match {
  view: PlayerViewDto | null;
  estado: Estado;
  aviso: string | null;
  jogar: (acao: ActionDto) => void;
}

/**
 * Conexão com uma partida, com reconexão.
 *
 * O cliente é burro de propósito: ele guarda a última view e o `seq` que ela
 * trouxe, e manda ação carimbada com esse `seq`. Se o servidor recusar por
 * `stale`, a resposta certa não é insistir — é pedir reidratação.
 *
 * Quando a conexão cai, ele volta sozinho. O estado mora no servidor, então
 * reconectar não perde nada; o `Welcome` traz a view atual e o `seq` de lá.
 */
export function useMatch(matchId: string | null, seat: number): Match {
  const [view, setView] = useState<PlayerViewDto | null>(null);
  const [estado, setEstado] = useState<Estado>('ocioso');
  const [aviso, setAviso] = useState<string | null>(null);

  const socket = useRef<WebSocket | null>(null);
  const seq = useRef(0);
  const tentativas = useRef(0);

  useEffect(() => {
    if (!matchId) return;

    let vivo = true;
    let timer: number | undefined;

    const enviar = (ws: WebSocket, mensagem: ClientMessage) => {
      ws.send(JSON.stringify(mensagem));
    };

    const conectar = () => {
      if (!vivo) return;

      setEstado('conectando');
      const esquema = location.protocol === 'https:' ? 'wss' : 'ws';
      const ws = new WebSocket(`${esquema}://${location.host}/ws/${matchId}`);
      socket.current = ws;

      ws.onopen = () => {
        tentativas.current = 0;
        enviar(ws, { type: 'join', protocol: PROTOCOL_VERSION, seat });
      };

      ws.onmessage = (evento) => {
        const mensagem = JSON.parse(evento.data) as ServerMessage;

        switch (mensagem.type) {
          case 'welcome':
          case 'view':
            seq.current = mensagem.seq;
            setView(mensagem.view);
            setEstado('conectado');
            setAviso(null);
            break;
          case 'rejected':
            setAviso(`ação recusada: ${mensagem.reason}`);
            enviar(ws, { type: 'resync', seq: seq.current });
            break;
          case 'error':
            setAviso(`erro do servidor: ${mensagem.kind}`);
            setEstado('encerrado');
            vivo = false;
            ws.close();
            break;
        }
      };

      ws.onclose = () => {
        if (!vivo) return;
        setEstado('caido');
        // Espera crescente, com teto: não adianta martelar um servidor que
        // ainda está subindo, e não adianta desistir de um que caiu um segundo.
        const espera = Math.min(500 * 2 ** tentativas.current, 5000);
        tentativas.current += 1;
        timer = window.setTimeout(conectar, espera);
      };

      ws.onerror = () => ws.close();
    };

    conectar();

    return () => {
      vivo = false;
      if (timer !== undefined) window.clearTimeout(timer);
      socket.current?.close();
      socket.current = null;
    };
  }, [matchId, seat]);

  const jogar = useCallback((acao: ActionDto) => {
    const ws = socket.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify({ type: 'action', seq: seq.current, action: acao }));
  }, []);

  return { view, estado, aviso, jogar };
}
