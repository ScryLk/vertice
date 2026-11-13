import { useEffect, useState } from 'react';

import { Ficha } from './Ficha';
import type { CardDefDto } from './protocol';
import init, { catalogo } from './wasm/vertice_wasm';

/**
 * O acervo inteiro na linguagem visual escolhida.
 *
 * Serve para julgar a direção com o dado real, não com mockup: se uma carta
 * do conjunto não se distingue das outras à primeira vista, o sistema está
 * errado e dá para ver aqui antes de qualquer linha de Pixi.
 */
export function Catalogo() {
  const [cartas, setCartas] = useState<CardDefDto[] | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    let vivo = true;

    (async () => {
      try {
        await init();
        if (!vivo) return;
        setCartas(JSON.parse(catalogo()) as CardDefDto[]);
      } catch (falha) {
        if (vivo) setErro(String(falha));
      }
    })();

    return () => {
      vivo = false;
    };
  }, []);

  if (erro) return <p className="aviso">Não consegui ler o catálogo: {erro}</p>;
  if (!cartas) return <p>Lendo o acervo…</p>;

  return (
    <>
      <p className="explicacao">
        {cartas.length} verbetes. Sem cor: forma, numeral e tipografia carregam tudo. O sigilo é
        gerado do dado da carta — moldura pelo tipo, traços pela mana, marca central pelo efeito —
        então carta nova ganha sigilo sozinha.
      </p>
      <div className="grade-fichas">
        {cartas.map((carta) => (
          <Ficha key={carta.id} carta={carta} />
        ))}
      </div>
      <p>
        <a href="?">voltar</a>
      </p>
    </>
  );
}
