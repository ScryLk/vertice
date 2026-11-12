import { useState } from 'react';

import { Duel } from './Duel';
import type { CreateMatchResponse } from './protocol';
import { useMatch } from './useMatch';

export default function App() {
  const params = new URLSearchParams(window.location.search);
  const matchId = params.get('match');
  const seat = Number(params.get('seat') ?? '0');

  if (!matchId) return <Lobby />;
  return <Mesa matchId={matchId} seat={seat} />;
}

function Mesa({ matchId, seat }: { matchId: string; seat: number }) {
  const { view, estado, aviso, jogar } = useMatch(matchId, seat);

  if (!view) {
    return (
      <div className="mesa">
        <p>
          Conectando à partida <code>{matchId}</code> como assento {seat}…
        </p>
        <p className={`estado estado-${estado}`}>{estado}</p>
        {aviso && <p className="aviso">{aviso}</p>}
      </div>
    );
  }

  return <Duel view={view} estado={estado} aviso={aviso} jogar={jogar} />;
}

function Lobby() {
  const [criando, setCriando] = useState(false);
  const [criada, setCriada] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  async function criar() {
    setCriando(true);
    setErro(null);
    try {
      const resposta = await fetch('/matches', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        // A semente vem do cliente porque é a única coisa que ele pode
        // escolher. O servidor grava no `MatchRecord`, então o replay é exato.
        body: JSON.stringify({ seed: Math.floor(Math.random() * 1_000_000) }),
      });
      if (!resposta.ok) throw new Error(`${resposta.status}: ${await resposta.text()}`);
      const dados = (await resposta.json()) as CreateMatchResponse;
      setCriada(dados.match_id);
    } catch (falha) {
      setErro(String(falha));
    } finally {
      setCriando(false);
    }
  }

  return (
    <div className="mesa lobby">
      <h1>Vértice</h1>
      <p className="explicacao">
        Duelo de teste, sem arte de propósito. Esta tela existe para provar o protocolo — se algo
        doer aqui, dói agora, enquanto ainda é barato mudar.
      </p>

      {criada ? (
        <>
          <p>
            Partida <code>{criada}</code> criada. Abra os dois assentos, em abas diferentes:
          </p>
          <ul>
            <li>
              <a href={`?match=${criada}&seat=0`}>assento 0</a>
            </li>
            <li>
              <a href={`?match=${criada}&seat=1`}>assento 1</a>
            </li>
          </ul>
        </>
      ) : (
        <button type="button" onClick={criar} disabled={criando}>
          {criando ? 'Criando…' : 'Criar partida'}
        </button>
      )}

      {erro && <p className="aviso">{erro}</p>}
    </div>
  );
}
