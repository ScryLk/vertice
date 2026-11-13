// A direção visual é uma decisão, e decisão sem teste degrada em silêncio.
//
// O que está protegido aqui é o que sustenta a direção: sem cor, a carta
// precisa ser reconhecível por **estrutura**. Se alguém tirar a moldura do
// sigilo, o corpo do rodapé ou a marca de catalogação, estas fichas deixam de
// se distinguir — e o defeito só apareceria com o pool grande.

import { cleanup, render } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { Ficha } from './Ficha';
import type { CardDefDto } from './protocol';

afterEach(cleanup);

function carta(over: Partial<CardDefDto> = {}): CardDefDto {
  return {
    id: 0,
    name: 'Carta',
    cost: 1,
    kind: 'creature',
    power: 1,
    toughness: 1,
    keywords: [],
    effect: { kind: 'none' },
    on_enter: null,
    ...over,
  };
}

describe('Ficha', () => {
  it('mostra registro, nome, tipo e custo', () => {
    const { container } = render(<Ficha carta={carta({ id: 42, name: 'Brasa Dupla', cost: 3 })} />);

    expect(container.querySelector('.registro')?.textContent).toContain('0042');
    expect(container.querySelector('.ficha-nome')?.textContent).toBe('Brasa Dupla');
    expect(container.querySelector('.ficha-tipo')?.textContent).toContain('criatura');
    expect(container.querySelector('.custo')?.textContent).toBe('3');
  });

  it('criatura mostra o corpo, terreno não', () => {
    const criatura = render(<Ficha carta={carta({ kind: 'creature', power: 4, toughness: 4 })} />);
    expect(criatura.container.querySelector('.corpo')?.textContent).toBe('4/4');
    cleanup();

    const terreno = render(<Ficha carta={carta({ kind: 'land', power: 0, toughness: 0 })} />);
    expect(terreno.container.querySelector('.corpo')?.textContent).toBe('');
  });

  it('o sigilo muda de moldura com o tipo', () => {
    const molduras = (['land', 'creature', 'spell'] as const).map((kind) => {
      const { container } = render(<Ficha carta={carta({ kind })} />);
      const moldura = container.querySelector('.sigilo path')?.getAttribute('d');
      cleanup();
      return moldura;
    });

    expect(new Set(molduras).size).toBe(3);
  });

  it('a marca de catalogação diferencia cartas de mesmo perfil', () => {
    // O sigilo descreve o que a carta faz. Duas cartas de perfil idêntico
    // teriam o mesmo sigilo — a marca é o que diz qual é qual.
    const marca = (id: number) => {
      const { container } = render(<Ficha carta={carta({ id })} />);
      const furados = [...container.querySelectorAll('.marca > span')].map((casa) =>
        casa.className.includes('furado') ? '1' : '0',
      );
      cleanup();
      return furados.join('');
    };

    expect(marca(0)).toBe('00000');
    expect(marca(1)).toBe('10000');
    expect(marca(3)).toBe('11000');
    expect(marca(5)).toBe('10100');

    const distintas = new Set([0, 1, 2, 3, 4, 5, 6, 7].map(marca));
    expect(distintas.size).toBe(8);
  });

  it('a magnitude do pump muda o glifo', () => {
    // O sigilo desenhava o *sinal* do pump e ignorava a magnitude: +2/+2 e
    // +4/+4 saíam idênticos, e -1/-1 com -3/-3 também. Descoberto olhando o
    // acervo com 50 cartas.
    const glifo = (power: number) => {
      const { container } = render(
        <Ficha
          carta={carta({ kind: 'spell', effect: { kind: 'pump', power, toughness: power } })}
        />,
      );
      const tracos = [...container.querySelectorAll('.sigilo path')];
      const marca = tracos[tracos.length - 1]?.getAttribute('d');
      cleanup();
      return marca;
    };

    expect(glifo(2)).not.toBe(glifo(4));
    expect(glifo(-1)).not.toBe(glifo(-3));
    expect(glifo(2)).not.toBe(glifo(-2));
  });

  it('o gatilho de entrada vira texto', () => {
    const { container } = render(
      <Ficha carta={carta({ on_enter: { kind: 'draw', amount: 1 } })} />,
    );
    expect(container.querySelector('.ficha-gatilho')?.textContent).toBe('Ao entrar, compre 1 carta.');
  });

  it('o sigilo muda quando a criatura tem gatilho', () => {
    // A lição de `direcao-visual.md`: um glifo tem que ser função de *tudo*
    // que distingue a carta. Duas criaturas de mesmo corpo e custo — uma com
    // compra ao entrar, outra sem — não podem sair idênticas.
    const glifo = (over: Partial<CardDefDto>) => {
      const { container } = render(<Ficha carta={carta(over)} />);
      const svg = container.querySelector('.sigilo')?.innerHTML;
      cleanup();
      return svg;
    };

    expect(glifo({})).not.toBe(glifo({ on_enter: { kind: 'draw', amount: 1 } }));
    expect(glifo({ on_enter: { kind: 'draw', amount: 1 } })).not.toBe(
      glifo({ on_enter: { kind: 'gain_life', amount: 1 } }),
    );
  });

  it('vida ganha vira texto, e a magnitude muda o glifo', () => {
    const vida = (amount: number) => {
      const { container } = render(
        <Ficha carta={carta({ kind: 'spell', effect: { kind: 'gain_life', amount } })} />,
      );
      const texto = container.querySelector('.ficha-efeito')?.textContent;
      const svg = container.querySelector('.sigilo')?.innerHTML;
      cleanup();
      return { texto, svg };
    };

    expect(vida(3).texto).toBe('Ganha 3 de vida.');
    expect(vida(2).svg).not.toBe(vida(8).svg);
  });

  it('o efeito vira texto legível', () => {
    const { container } = render(
      <Ficha carta={carta({ kind: 'spell', effect: { kind: 'deal_damage', amount: 3 } })} />,
    );
    expect(container.querySelector('.ficha-efeito')?.textContent).toBe('Causa 3 de dano.');
  });
});
