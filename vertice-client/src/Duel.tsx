import { useAcervo, type Acervo } from './acervo';
import { resumoDaCarta } from './Ficha';
import {
  nomeDaPalavraChave,
  type ActionDto,
  type CardDefDto,
  type CardViewDto,
  type PlayerViewDto,
  type TargetDto,
} from './protocol';
import type { Estado } from './useMatch';

interface Props {
  view: PlayerViewDto;
  estado: Estado;
  aviso: string | null;
  jogar: (acao: ActionDto) => void;
}

function nomeDoObjeto(view: PlayerViewDto, object: number): string {
  const visiveis = [...view.you.hand, ...view.you.battlefield, ...view.opponent.battlefield];
  return visiveis.find((carta) => carta.object === object)?.name ?? `objeto ${object}`;
}

/**
 * O alvo pelo nome, não pelo id.
 *
 * `objeto 30` não diz nada a quem está jogando; `Corvo do Arquivo` diz. Quando
 * a criatura não está visível — e só então — cai para o id, porque inventar um
 * nome seria pior que assumir que não se sabe.
 */
function nomeDoAlvo(view: PlayerViewDto, target: TargetDto): string {
  return target.type === 'player' ? `jogador ${target.player}` : nomeDoObjeto(view, target.object);
}

function rotuloDaAcao(view: PlayerViewDto, acao: ActionDto): string {
  switch (acao.type) {
    case 'play_land':
      return `Baixar ${nomeDoObjeto(view, acao.object)}`;
    case 'cast':
      return acao.target
        ? `Conjurar ${nomeDoObjeto(view, acao.object)} → ${nomeDoAlvo(view, acao.target)}`
        : `Conjurar ${nomeDoObjeto(view, acao.object)}`;
    case 'attack':
      return `Atacar com ${nomeDoObjeto(view, acao.attacker)} → ${nomeDoAlvo(view, acao.target)}`;
    case 'pass':
      return 'Passar';
    case 'end_turn':
      return 'Encerrar turno';
  }
}

function Carta({
  carta,
  minha,
  def,
}: {
  carta: CardViewDto;
  minha: boolean;
  def?: CardDefDto;
}) {
  const marcas = carta.keywords.map(nomeDaPalavraChave);
  if (carta.damage > 0) marcas.push(`${carta.damage} de dano`);
  if (carta.tapped) marcas.push('virada');

  // O texto da carta só aparece onde ele decide alguma coisa: na mão. No campo
  // a linha viraria ruído, e o que importa ali já está nas marcas.
  const efeito = def ? resumoDaCarta(def) : null;

  return (
    <li className={`carta ${minha ? 'minha' : 'alheia'} ${carta.tapped ? 'virada' : ''}`}>
      <span className="nome">{carta.name}</span>
      {/* Corpo só quando existe. "0/0" embaixo de uma mágica não é informação,
          é ruído — e com o texto da carta logo abaixo, ruído atrapalha. */}
      {(carta.power > 0 || carta.toughness > 0) && (
        <span className="corpo">
          {carta.power}/{carta.toughness}
        </span>
      )}
      {marcas.length > 0 && <span className="marcas">{marcas.join(' · ')}</span>}
      {efeito && <span className="efeito">{efeito}</span>}
    </li>
  );
}

function Zona({
  titulo,
  cartas,
  minha,
  acervo,
}: {
  titulo: string;
  cartas: CardViewDto[];
  minha: boolean;
  acervo?: Acervo | null;
}) {
  return (
    <section className="zona">
      <h3>
        {titulo} <small>({cartas.length})</small>
      </h3>
      {cartas.length === 0 ? (
        <p className="vazio">—</p>
      ) : (
        <ul className="cartas">
          {cartas.map((carta) => (
            <Carta
              key={carta.object}
              carta={carta}
              minha={minha}
              def={acervo?.get(carta.card)}
            />
          ))}
        </ul>
      )}
    </section>
  );
}

export function Duel({ view, estado, aviso, jogar }: Props) {
  const encerrada = view.winner !== null;
  const acervo = useAcervo();

  return (
    <div className="mesa">
      <header className="topo">
        <span>
          turno {view.turn} · {view.phase === 'main' ? 'principal' : 'reação'} · prioridade{' '}
          {view.priority}
        </span>
        <span className={`estado estado-${estado}`}>{estado}</span>
      </header>

      {aviso && <p className="aviso">{aviso}</p>}

      {encerrada && (
        <p className="fim">
          {view.winner === view.you.id ? 'Você venceu.' : 'Você perdeu.'}
        </p>
      )}

      <section className="lado oponente">
        <h2>
          Oponente · {view.opponent.life} de vida · {view.opponent.hand_count} na mão ·{' '}
          {view.opponent.library_count} no baralho
        </h2>
        <Zona titulo="Campo do oponente" cartas={view.opponent.battlefield} minha={false} />
        <p className="vazio">cemitério: {view.opponent.graveyard.length}</p>
      </section>

      {view.stack.length > 0 && (
        <section className="pilha">
          <h3>Pilha</h3>
          <ol>
            {view.stack.map((entrada, indice) => (
              <li key={indice}>
                {entrada.name} (assento {entrada.controller})
              </li>
            ))}
          </ol>
        </section>
      )}

      <section className="lado voce">
        <h2>
          Você · {view.you.life} de vida · {view.you.mana} de mana · {view.you.library_count} no
          baralho
        </h2>
        <Zona titulo="Seu campo" cartas={view.you.battlefield} minha />
        <Zona titulo="Sua mão" cartas={view.you.hand} minha acervo={acervo} />
      </section>

      <section className="acoes">
        <h3>Ações legais</h3>
        {encerrada ? (
          <p className="vazio">A partida acabou.</p>
        ) : view.legal_actions.length === 0 ? (
          <p className="vazio">Nenhuma ação — é a vez do outro assento.</p>
        ) : (
          <div className="botoes">
            {view.legal_actions.map((acao, indice) => (
              <button key={indice} type="button" onClick={() => jogar(acao)}>
                {rotuloDaAcao(view, acao)}
              </button>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
