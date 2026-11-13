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
      return `Compre ${effect.amount} ${effect.amount === 1 ? 'carta' : 'cartas'}.`;
    case 'gain_life':
      return `Ganha ${effect.amount} de vida.`;
    case 'lose_life':
      return `O oponente perde ${effect.amount} de vida.`;
    case 'destroy':
      return 'Destrói a criatura alvo.';
    case 'pump':
      return `${effect.power >= 0 ? '+' : ''}${effect.power}/${effect.toughness >= 0 ? '+' : ''}${effect.toughness}.`;
  }
}

/**
 * O gatilho de entrada em texto.
 *
 * Separado de `textoDoEfeito` porque a frase muda de sujeito: "Causa 3 de
 * dano" vira "Ao entrar, causa 3 de dano", e juntar as duas com concatenação
 * produziria "Ao entrar, Compre 1 carta". Duas funções pequenas e certas valem
 * mais que uma genérica e torta.
 */
function textoDoGatilho(effect: EffectDto, quando: 'Ao entrar' | 'Ao morrer'): string | null {
  switch (effect.kind) {
    case 'draw':
      return `${quando}, compre ${effect.amount} ${effect.amount === 1 ? 'carta' : 'cartas'}.`;
    case 'gain_life':
      return `${quando}, ganhe ${effect.amount} de vida.`;
    case 'lose_life':
      return `${quando}, o oponente perde ${effect.amount} de vida.`;
    case 'deal_damage':
      return `${quando}, cause ${effect.amount} de dano a um alvo.`;
    case 'destroy':
      return `${quando}, destrua a criatura alvo.`;
    case 'pump':
      return `${quando}, uma criatura recebe ${effect.power >= 0 ? '+' : ''}${effect.power}/${
        effect.toughness >= 0 ? '+' : ''
      }${effect.toughness}.`;
    default:
      // O validador só deixa passar estes no gatilho. Se um dia chegar outro
      // aqui, a ficha cala em vez de mentir.
      return null;
  }
}

/**
 * Tudo o que a carta faz, numa linha.
 *
 * Para onde não cabe a ficha inteira — a mão do duelo. Reaproveita as mesmas
 * frases da ficha de propósito: duas redações para a mesma carta divergem, e a
 * que fica escondida é sempre a que mente.
 */
export function resumoDaCarta(carta: CardDefDto): string | null {
  const partes = [
    textoDoEfeito(carta.effect),
    carta.on_enter ? textoDoGatilho(carta.on_enter, 'Ao entrar') : null,
    carta.on_die ? textoDoGatilho(carta.on_die, 'Ao morrer') : null,
  ].filter((parte): parte is string => parte !== null);

  return partes.length > 0 ? partes.join(' ') : null;
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
  // A marca central mostra o que a carta *faz* — e criatura não tem efeito
  // próprio: o que ela faz é o gatilho. Sem esta troca, duas criaturas de
  // mesmo corpo e gatilhos diferentes sairiam com o mesmo glifo, que é
  // exatamente o defeito que já apareceu com −1/−1 e −3/−3.
  const marca: EffectDto =
    carta.effect.kind !== 'none'
      ? carta.effect
      : (carta.on_enter ?? carta.on_die ?? carta.effect);
  const magnitude = marca.kind === 'pump' ? Math.abs(marca.power) : 0;
  const forcaDoPump = Math.min(26, 8 + magnitude * 5);
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

      {marca.kind === 'none' && <circle cx="50" cy="50" r="3" fill="currentColor" />}
      {marca.kind === 'deal_damage' &&
        Array.from({ length: marca.amount }, (_, indice) => (
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
      {marca.kind === 'draw' &&
        Array.from({ length: marca.amount }, (_, indice) => (
          <circle key={indice} cx={42 + indice * 16} cy={50} r="5" fill="none" stroke="currentColor" strokeWidth="2" />
        ))}
      {marca.kind === 'destroy' && (
        // Quadrado cheio: é a única marca preenchida e angular do sistema, e
        // por isso não disputa forma com ponto, anel, losango, barras ou cruz.
        // Sem magnitude porque destruir não tem tamanho — mata ou não mata.
        <rect x="41" y="41" width="18" height="18" fill="currentColor" />
      )}
      {marca.kind === 'gain_life' && (
        // Losango, e o tamanho carrega a magnitude — mesma lição do pump:
        // glifo que ignora número faz duas cartas diferentes parecerem iguais.
        //
        // Losango e não anel: o anel de palavra-chave já mora no mesmo centro,
        // e com 1 palavra-chave e 2 de vida os dois caíam no mesmo raio. Visto
        // no acervo com 66 cartas — a mesma colisão de glifo que já apareceu
        // com +2/+2 e +4/+4, só que entre eixos diferentes.
        <path
          d={(() => {
            const r = Math.min(14, 6 + marca.amount);
            return `M50 ${50 - r} L${50 + r} 50 L50 ${50 + r} L${50 - r} 50 Z`;
          })()}
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
        />
      )}
      {marca.kind === 'pump' && (
        // O comprimento do traço carrega a magnitude. Sem isso, +2/+2 e +4/+4
        // ficam com o mesmo glifo — visível no acervo real, e o sigilo deixa
        // de dizer o que a carta faz.
        <path
          d={
            marca.power < 0
              ? `M${50 - forcaDoPump} 50 H${50 + forcaDoPump}`
              : `M50 ${50 - forcaDoPump} V${50 + forcaDoPump} M${50 - forcaDoPump} 50 H${50 + forcaDoPump}`
          }
          stroke="currentColor"
          strokeWidth="3"
          fill="none"
        />
      )}

      {/*
        Gatilhos: meio anel em cima para quando entra, meio anel embaixo para
        quando morre. A leitura é a do próprio campo de batalha — em cima é
        onde a criatura fica, embaixo é para onde ela vai. Uma carta com os
        dois gatilhos desenha o anel inteiro, e a metade que falta diz qual
        dos dois ela tem. Sólido porque tracejado já significa palavra-chave.
      */}
      {carta.on_enter && (
        <path d="M2 50 A48 48 0 0 1 98 50" fill="none" stroke="currentColor" strokeWidth="2" />
      )}
      {carta.on_die && (
        <path d="M98 50 A48 48 0 0 1 2 50" fill="none" stroke="currentColor" strokeWidth="2" />
      )}

      {carta.keywords.map((palavra, indice) => (
        // A partir de 18, e não de 10: a faixa de 3 a 13 do centro é da marca
        // do efeito — desenho de compra e losango de vida moram ali. Com o
        // raio antigo o anel de palavra-chave caía em cima deles, e duas
        // cartas que só diferem na palavra-chave saíam iguais.
        <circle
          key={palavra}
          cx={50}
          cy={50}
          r={18 + indice * 5}
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
  const aoEntrar = carta.on_enter ? textoDoGatilho(carta.on_enter, 'Ao entrar') : null;
  const aoMorrer = carta.on_die ? textoDoGatilho(carta.on_die, 'Ao morrer') : null;
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
      {aoEntrar && <p className="ficha-efeito ficha-gatilho ficha-gatilho-entrada">{aoEntrar}</p>}
      {aoMorrer && <p className="ficha-efeito ficha-gatilho ficha-gatilho-morte">{aoMorrer}</p>}

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
