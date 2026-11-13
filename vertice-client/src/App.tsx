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

async function criarPartida(corpo: Record<string, unknown>): Promise<string> {
  const resposta = await fetch('/matches', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    // A semente vem do cliente porque é a única coisa que ele pode escolher.
    // O servidor grava no `MatchRecord`, então o replay é exato.
    body: JSON.stringify({ seed: Math.floor(Math.random() * 1_000_000), ...corpo }),
  });

  if (!resposta.ok) throw new Error(`${resposta.status}: ${await resposta.text()}`);

  const dados = (await resposta.json()) as CreateMatchResponse;
  return dados.match_id;
}

function Lobby() {
  const [ocupado, setOcupado] = useState<'nenhum' | 'dois' | 'bot'>('nenhum');
  const [criada, setCriada] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  async function doisJogadores() {
    setOcupado('dois');
    setErro(null);
    try {
      setCriada(await criarPartida({}));
    } catch (falha) {
      setErro(String(falha));
    } finally {
      setOcupado('nenhum');
    }
  }

  async function contraOBot(nivel: 'calouro' | 'formando') {
    setOcupado('bot');
    setErro(null);
    try {
      const id = await criarPartida({ level: nivel });
      window.location.search = `?match=${id}&seat=0`;
    } catch (falha) {
      setErro(String(falha));
      setOcupado('nenhum');
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
        <div className="botoes">
          <button type="button" onClick={doisJogadores} disabled={ocupado !== 'nenhum'}>
            {ocupado === 'dois' ? 'Criando…' : 'Dois jogadores'}
          </button>
          <button type="button" onClick={() => contraOBot('calouro')} disabled={ocupado !== 'nenhum'}>
            {ocupado === 'bot' ? 'Criando…' : 'Jogar contra o calouro'}
          </button>
          <button type="button" onClick={() => contraOBot('formando')} disabled={ocupado !== 'nenhum'}>
            Jogar contra o formando
          </button>
        </div>
      )}

      {erro && <p className="aviso">{erro}</p>}
    </div>
  );
}
