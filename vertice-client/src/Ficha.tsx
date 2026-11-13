import type { CardDefDto, EffectDto, KeywordDto } from './protocol';

/**
 * A carta como verbete do Arquivo.
 *
 * Sem cor: a ficha é tinta sobre papel, e toda a informação vem de estrutura
 * e tipografia. Duas consequências que valem registrar:
 *
 * 1. **Reconhecimento não pode depender de cor**, então o *sigilo* carrega
 *    isso. Ele é gerado do dado da carta — moldura pelo tipo, traços pela
 *    quantidade de mana, marca central pelo efeito. Carta nova ganha sigilo
 *    sozinha; não existe 250º desenho para encomendar.
 * 2. **O papel é o objeto.** A mesa é pedra escura; a ficha é papel claro
 *    pousado nela. O peso emocional do duelo vem da mesa, não da carta.
 */

const NOME_DO_TIPO: Record<CardDefDto['kind'], string> = {
  land: 'terreno',
  creature: 'criatura',
  spell: 'mágica',
};

const NOME_DA_PALAVRA: Record<KeywordDto, string> = {
  flying: 'voar',
  vigilance: 'vigilância',
  deathtouch: 'toque letal',
};

function textoDoEfeito(effect: EffectDto): string | null {
  switch (effect.kind) {
    case 'none':
      return null;
    case 'deal_damage':
      return `Causa ${effect.amount} de dano.`;
    case 'draw':
      return `Compre ${effect.amount} cartas.`;
    case 'pump':
      return `${effect.power >= 0 ? '+' : ''}${effect.power}/${effect.toughness >= 0 ? '+' : ''}${effect.toughness}.`;
  }
}

function numeroDeCatalogo(id: number): string {
  return String(id).padStart(4, '0');
}

/**
 * O sigilo: a carta inteira comprimida num glifo.
 *
 * Determinístico e derivado só do dado — mesma carta, mesmo glifo, sempre.
 * É o que permite reconhecer uma carta de relance sem cor e sem ilustração.
 */
function Sigilo({ carta }: { carta: CardDefDto }) {
  const tracos = carta.cost;
  const moldura =
    carta.kind === 'land'
      ? 'M20 20 H80 V80 H20 Z'
      : carta.kind === 'creature'
        ? 'M50 16 L82 33 V67 L50 84 L18 67 V33 Z'
        : 'M50 16 A34 34 0 1 1 49.9 16 Z';

  return (
    <svg className="sigilo" viewBox="0 0 100 100" aria-hidden="true">
      <path d={moldura} fill="none" stroke="currentColor" strokeWidth="2" />

      {Array.from({ length: tracos }, (_, indice) => {
        const angulo = (indice / Math.max(tracos, 1)) * Math.PI * 2 - Math.PI / 2;
        const x1 = 50 + Math.cos(angulo) * 40;
        const y1 = 50 + Math.sin(angulo) * 40;
        const x2 = 50 + Math.cos(angulo) * 46;
        const y2 = 50 + Math.sin(angulo) * 46;
        return <line key={indice} x1={x1} y1={y1} x2={x2} y2={y2} stroke="currentColor" strokeWidth="2" />;
      })}

      {carta.effect.kind === 'none' && <circle cx="50" cy="50" r="3" fill="currentColor" />}
      {carta.effect.kind === 'deal_damage' &&
        Array.from({ length: carta.effect.amount }, (_, indice) => (
          <line
            key={indice}
            x1={38 + indice * 8}
            y1={40}
            x2={44 + indice * 8}
            y2={60}
            stroke="currentColor"
            strokeWidth="3"
          />
        ))}
      {carta.effect.kind === 'draw' &&
        Array.from({ length: carta.effect.amount }, (_, indice) => (
          <circle key={indice} cx={42 + indice * 16} cy={50} r="5" fill="none" stroke="currentColor" strokeWidth="2" />
        ))}
      {carta.effect.kind === 'pump' && (
        <path
          d={carta.effect.power < 0 ? 'M38 50 H62' : 'M50 36 V64 M38 50 H62'}
          stroke="currentColor"
          strokeWidth="3"
          fill="none"
        />
      )}

      {carta.keywords.map((palavra, indice) => (
        <circle
          key={palavra}
          cx={50}
          cy={50}
          r={10 + indice * 5}
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          strokeDasharray="2 3"
        />
      ))}
    </svg>
  );
}

export function Ficha({ carta, escala = 1 }: { carta: CardDefDto; escala?: number }) {
  const efeito = textoDoEfeito(carta.effect);
  const palavras = carta.keywords.map((palavra) => NOME_DA_PALAVRA[palavra]);
  const ehCriatura = carta.kind === 'creature';

  return (
    <article
      className={`ficha ficha-${carta.kind}`}
      style={{ '--escala': escala } as React.CSSProperties}
    >
      <header className="ficha-topo">
        <span className="registro">ARQUIVO · {numeroDeCatalogo(carta.id)}</span>
        <span className="custo">{carta.kind === 'land' ? '—' : carta.cost}</span>
      </header>

      <h2 className="ficha-nome">{carta.name}</h2>

      <div className="ficha-sigilo">
        <Sigilo carta={carta} />
      </div>

      <p className="ficha-tipo">
        {NOME_DO_TIPO[carta.kind]}
        {palavras.length > 0 && <span className="palavras"> · {palavras.join(' · ')}</span>}
      </p>

      {efeito && <p className="ficha-efeito">{efeito}</p>}

      <footer className="ficha-rodape">
        <span className="selo" aria-hidden="true">
          {carta.kind === 'land' ? '◻' : carta.kind === 'creature' ? '◆' : '◇'}
        </span>
        {/*
          Marca de catalogação: cinco casas furadas, derivadas do id. O sigilo
          diz o que a carta *faz* — tipo, custo, efeito —, e por isso duas
          cartas de perfil igual teriam o mesmo sigilo. A marca diz *qual* é.
          Sem ela, um acervo de 250 fichas teria dezenas de gêmeas.
        */}
        <span className="marca" aria-hidden="true">
          {Array.from({ length: 5 }, (_, bit) => (
            <span key={bit} className={(carta.id >> bit) & 1 ? 'furado' : 'vazio'} />
          ))}
        </span>
        {ehCriatura ? (
          <span className="corpo">
            {carta.power}/{carta.toughness}
          </span>
        ) : (
          <span className="corpo vazio" />
        )}
      </footer>
    </article>
  );
}
