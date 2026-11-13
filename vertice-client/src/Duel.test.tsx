// Teste de renderização, não de comportamento.
//
// O servidor já prova que a partida roda: `partida_joga_ate_o_fim` em
// `vertice-server/tests/reconnect.rs` joga até alguém vencer, por WebSocket.
// O que aquele teste não vê é o que sai na tela — se um tipo de ação não vira
// botão, se a palavra-chave some, se a mão do oponente escorrega.
//
// É isso que está aqui, e é por isso que as visões são fixas: o alvo é a
// renderização, não a regra.

import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { Duel } from './Duel';
import type { CardViewDto, PlayerViewDto } from './protocol';

// O catálogo real vem do WASM; aqui ele é fixo. O que se prova nesta suíte é a
// renderização, e o catálogo é a única dependência que o teste não controla.
vi.mock('./acervo', () => ({
  useAcervo: () =>
    new Map([
      [
        7,
        {
          id: 7,
          name: 'Faísca',
          cost: 1,
          kind: 'spell',
          power: 0,
          toughness: 0,
          keywords: [],
          effect: { kind: 'deal_damage', amount: 2 },
          on_enter: null,
          on_die: null,
        },
      ],
      [
        8,
        {
          id: 8,
          name: 'Arqueiro do Pátio',
          cost: 2,
          kind: 'creature',
          power: 1,
          toughness: 2,
          keywords: [],
          effect: { kind: 'none' },
          on_enter: { kind: 'deal_damage', amount: 2 },
          on_die: null,
        },
      ],
    ]),
}));

afterEach(cleanup);

function carta(campos: Partial<CardViewDto> & { object: number; name: string }): CardViewDto {
  return {
    card: 0,
    power: 0,
    toughness: 0,
    tapped: false,
    damage: 0,
    keywords: [],
    ...campos,
  };
}

function visao(over: Partial<PlayerViewDto> = {}): PlayerViewDto {
  return {
    you: {
      id: 0,
      life: 40,
      hand: [],
      library_count: 90,
      battlefield: [],
      graveyard: [],
      mana: 0,
    },
    opponent: {
      id: 1,
      life: 40,
      hand_count: 7,
      library_count: 90,
      battlefield: [],
      graveyard: [],
    },
    turn: 1,
    phase: 'main',
    priority: 0,
    stack: [],
    legal_actions: [],
    winner: null,
    ...over,
  };
}

function mostrar(view: PlayerViewDto, jogar = vi.fn()) {
  render(<Duel view={view} estado="conectado" aviso={null} jogar={jogar} />);
  return jogar;
}

/// Uma carta de cada tipo de ação, todas visíveis, para os rótulos poderem
/// mostrar nome em vez de id.
const COM_ACOES = visao({
  you: {
    id: 0,
    life: 40,
    mana: 1,
    library_count: 90,
    graveyard: [],
    hand: [
      carta({ object: 10, name: 'Várzea de Cruzilha' }),
      carta({ object: 11, card: 7, name: 'Faísca' }),
    ],
    battlefield: [carta({ object: 20, name: 'Cão de Dívida', power: 2, toughness: 2 })],
  },
  opponent: {
    id: 1,
    life: 40,
    hand_count: 7,
    library_count: 90,
    graveyard: [],
    battlefield: [carta({ object: 30, name: 'Corvo do Arquivo', power: 2, toughness: 1 })],
  },
  legal_actions: [
    { type: 'play_land', object: 10 },
    { type: 'cast', object: 11, target: { type: 'object', object: 30 } },
    { type: 'attack', attacker: 20, target: { type: 'player', player: 1 } },
    { type: 'pass' },
    { type: 'end_turn' },
  ],
});

describe('Duel', () => {
  it('mostra as duas vidas e a contagem de mão do oponente', () => {
    mostrar(visao());

    expect(screen.getByRole('heading', { name: /Oponente · 40 de vida · 7 na mão/ })).toBeTruthy();
    expect(screen.getByRole('heading', { name: /Você · 40 de vida/ })).toBeTruthy();
  });

  it('nunca renderiza a mão do oponente como cartas, só como número', () => {
    mostrar(visao());

    const oponente = document.querySelector('.oponente');
    expect(oponente).not.toBeNull();
    expect(oponente!.querySelectorAll('li')).toHaveLength(0);
    expect(oponente!.textContent).toContain('7 na mão');
  });

  it('gera um botão por ação legal, com nome de carta no rótulo', () => {
    mostrar(COM_ACOES);

    const rotulos = screen.getAllByRole('button').map((botao) => botao.textContent);
    expect(rotulos).toEqual([
      'Baixar Várzea de Cruzilha',
      'Conjurar Faísca → Corvo do Arquivo',
      'Atacar com Cão de Dívida → jogador 1',
      'Passar',
      'Encerrar turno',
    ]);
  });

  it('a mão mostra o que a carta faz', () => {
    // A `PlayerView` carrega o id da carta, não o texto. Sem o catálogo, a mão
    // mostra nome e corpo e o jogador conjura no escuro.
    const { container } = render(
      <Duel view={COM_ACOES} estado="conectado" aviso={null} jogar={vi.fn()} />,
    );
    expect(screen.getByText('Causa 2 de dano.')).toBeTruthy();

    // E sem corpo inventado: nem o terreno nem a mágica têm 0/0 para mostrar.
    const mao = [...container.querySelectorAll('.zona')].find((zona) =>
      zona.querySelector('h3')?.textContent?.startsWith('Sua mão'),
    );
    expect(mao?.querySelectorAll('.corpo')).toHaveLength(0);
  });

  it('a mão mostra o gatilho, que é o que o alvo do botão vai fazer', () => {
    mostrar(
      visao({
        you: {
          id: 0,
          life: 40,
          mana: 2,
          library_count: 90,
          graveyard: [],
          hand: [
            carta({ object: 12, card: 8, name: 'Arqueiro do Pátio', power: 1, toughness: 2 }),
          ],
          battlefield: [],
        },
      }),
    );
    expect(screen.getByText('Ao entrar, cause 2 de dano a um alvo.')).toBeTruthy();
  });

  it('clicar manda a ação exata que veio do servidor', () => {
    const jogar = mostrar(COM_ACOES);

    fireEvent.click(screen.getByText('Atacar com Cão de Dívida → jogador 1'));

    expect(jogar).toHaveBeenCalledWith({
      type: 'attack',
      attacker: 20,
      target: { type: 'player', player: 1 },
    });
  });

  it('com vencedor não sobra botão nenhum', () => {
    mostrar(visao({ winner: 0, legal_actions: [{ type: 'end_turn' }] }));

    expect(screen.queryAllByRole('button')).toHaveLength(0);
    expect(screen.getByText('Você venceu.')).toBeTruthy();
  });

  it('quem perdeu vê que perdeu', () => {
    mostrar(visao({ winner: 1 }));

    expect(screen.getByText('Você perdeu.')).toBeTruthy();
  });

  it('sem ação legal, avisa de quem é a vez em vez de mostrar botão', () => {
    mostrar(visao());

    expect(screen.queryAllByRole('button')).toHaveLength(0);
    expect(screen.getByText(/é a vez do outro assento/)).toBeTruthy();
  });

  it('mostra palavra-chave, dano acumulado e carta virada', () => {
    mostrar(
      visao({
        you: {
          id: 0,
          life: 40,
          mana: 0,
          library_count: 90,
          graveyard: [],
          hand: [],
          battlefield: [
            carta({
              object: 20,
              name: 'Corvo do Arquivo',
              power: 2,
              toughness: 1,
              keywords: ['flying'],
              damage: 1,
              tapped: true,
            }),
          ],
        },
      }),
    );

    const item = document.querySelector('.voce .carta');
    expect(item).not.toBeNull();
    expect(item!.textContent).toContain('voar');
    expect(item!.textContent).toContain('1 de dano');
    expect(item!.textContent).toContain('virada');
    expect(item!.className).toContain('virada');
  });

  it('mostra a pilha quando há mágica esperando resolução', () => {
    mostrar(visao({ stack: [{ controller: 1, name: 'Faísca' }] }));

    expect(screen.getByText(/Faísca \(assento 1\)/)).toBeTruthy();
  });
});
