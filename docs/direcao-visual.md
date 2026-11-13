# Direção visual

Documento de decisão. Diferente de `arquitetura.md` (que registra o porquê
técnico), este registra **o que o jogo se parece** e o que isso custa. Escrito
antes da primeira linha de Pixi, de propósito: camada visual refeita é caro.

---

## A decisão

**Verbete em tudo. Sem cor.**

A carta é uma ficha do Arquivo: papel, tinta, número de catálogo, tipografia.
Nenhuma ilustração por carta, nenhuma cor de identidade. Toda a informação vem
de **estrutura, numeral e tipografia**, e o reconhecimento de relance vem de
um **sigilo gerado do dado da carta**.

Uma linguagem, um track de produção. Nada é desenhado à mão por carta.

## Por que isso, e não ilustração

O cânone entregou o sistema pronto. O pátio do Vértice é coberto de
**assinaturas gravadas como nomes numa calçada** — o aluno não compra uma
lenda, ele sintoniza um sulco. E a escola é um arquivo: a Casa Arquivo cataloga
ecos, a Baluarte mantém "o melhor arquivo de quem fez o quê", a burocracia tem
protocolo e formulário até para quem virou vaso.

Um jogo sobre uma instituição que **cataloga assinaturas** não precisa de
trezentas pinturas. Precisa de um acervo que pareça um acervo.

E o número fecha: 200–250 cartas no mínimo para decks-padrão funcionarem
(`arquitetura.md` §6), e um jogo de 300 ilustrações é impossível sozinho
(`§9`). A direção não é uma preferência estética — é a única que cabe.

## O sistema

### Anatomia da ficha

```
┌────────────────────────────────┐
│ ARQUIVO · 0042            ３   │  registro (mono, fraco) · custo (mono, grande)
├────────────────────────────────┤
│ Brasa Dupla                    │  nome em small caps, serifada
│                                │
│            ╭───╮               │  sigilo: gerado do dado
│            │ ╱╱╱│              │
│            ╰───╯               │
│                                │
│ MÁGICA · FOGO                  │  tipo em mono, versalete, fraco
│ Causa 3 de dano.               │  efeito em texto, serifado
├────────────────────────────────┤
│ ◇   ▪▫▪▫▪            3/3      │  selo · marca de catalogação · corpo
└────────────────────────────────┘
```

- **Papel sobre pedra.** A mesa é escura; a ficha é papel claro pousado nela.
  O peso do duelo vem da mesa, não da carta — é o que resolve o risco de a
  tela ficar fria.
- **Duas famílias tipográficas.** Serifa para o que se lê (nome, efeito),
  monoespaçada para o que se confere (registro, custo, corpo). Nada de uma
  terceira fonte; cada uma a mais é uma decisão a mais por carta.
- **Estrutura muda com o tipo.** Terreno não tem corpo; criatura tem o corpo
  grande no rodapé; mágica tem o efeito em destaque. A diferença é de forma,
  nunca de cor.

### O sigilo

Gerado do dado, determinístico, sem desenho por carta:

| Elemento | Codifica |
|---|---|
| moldura | tipo — quadrado (terreno), hexágono (criatura), círculo (mágica) |
| traços radiais | custo, um por mana |
| marca central | efeito — ponto (nenhum), barras (dano), círculos (compra), traço (pump), losango (vida) |
| anel pontilhado | palavra-chave, um anel por palavra |
| anel sólido por fora | gatilho de entrada |

Quando a carta é criatura com gatilho, a **marca central desenha o efeito do
gatilho**, porque é isso que a carta faz: criatura não tem efeito próprio.

Carta nova ganha sigilo sozinha. O custo marginal de uma carta é **zero**.

### A marca de catalogação

Cinco casas furadas no rodapé, derivadas do `CardId` em binário.

Existe porque o sigilo descreve o que a carta **faz**, não qual ela **é**: duas
cartas de mesmo tipo, custo, efeito e palavras teriam sigilo idêntico. Com 13
cartas isso quase não acontece; com 250, acontece muito. A marca é a impressão
digital do verbete — e é o tipo de detalhe que um acervo de verdade tem.

## Os dois riscos, e o que os segura

**"Institucional até demais."** A ficha é fria por construção. O que carrega o
calor é o outro lado da tela: a mesa é pedra escura, o Vértice é um anfiteatro
onde leylines convergem, e é lá que entra movimento, luz e som. A ficha não
precisa ser bonita — precisa ser **legível e confiável**, como um documento
que o jogador já viu mil vezes.

Isso está implementado em `vertice-client/src/cenario/`: as leylines abrem do
fora para o centro em ~1,1s quando o duelo começa, e o núcleo pulsa depois.
Tudo geometria gerada, com semente fixa — cena que muda a cada recarregamento
parece defeito, não variedade. Os painéis da interface são translúcidos de
propósito: caixa opaca sobre cenário é a forma mais rápida de o cenário
deixar de existir.

**"Sem cor, perde legibilidade."** Reconhecimento não pode depender de cor,
então depende de forma: moldura do sigilo, número do custo em posição fixa,
corpo no canto do rodapé, marca de catalogação. Isso foi verificado com o
acervo real — ver `?cartas` no cliente.

## O que continua sendo arte paga

A direção iconográfica **não** elimina orçamento de arte; ela o concentra em
dois lugares, como a `arquitetura.md` §9 já pedia:

1. **Retrato dos comandantes.** É o que o jogador olha, e é o que dá rosto ao
   eco. Poucas peças, alta qualidade.
2. **A transição do overworld para o duelo.** É o momento de maior valor de
   produção do jogo.

Chibi, cenário e interface continuam terceirizáveis por peça.

## O que esta decisão proíbe

- Ilustração por carta. Nenhuma exceção "só nesta".
- Cor como portadora de informação — nem raridade, nem tipo, nem casa.
- Uma terceira família tipográfica.
- Textura ou ornamento que não codifique nada.

## Quando revisar

**Revisado com 50 cartas**, olhando o acervo inteiro em `?cartas`. O sistema
aguentou, e encontrou um defeito: o sigilo desenhava o *sinal* do pump e
ignorava a magnitude, então +2/+2 e +4/+4 saíam idênticos — e −1/−1 com −3/−3
também. Corrigido fazendo o comprimento do traço carregar a magnitude, com
teste cobrindo.

A lição vale para os próximos eixos: **um glifo tem que ser função de tudo o
que distingue a carta**, não só do que a nomeia. Se um número muda o
comportamento, ele tem que mudar o desenho.

**Revisado com 66 cartas, quando a DSL ganhou o gatilho de entrada.** O sistema
aguentou de novo, e de novo achou colisão — a mesma classe de defeito, num eixo
novo. O anel de palavra-chave nascia no raio 10 e a marca do efeito mora na
faixa de 3 a 13 do centro: criatura com uma palavra-chave e "ao entrar, ganhe 2
de vida" desenhava o anel *em cima* do losango. Pior, o glifo da compra já
caía dentro desse raio desde antes — o gatilho só tornou o caso comum.

Corrigido em duas partes: o anel de palavra-chave passou a nascer no raio 18, e
a marca de vida virou **losango** em vez de anel, para não disputar a mesma
forma com ele. Com teste cobrindo que criaturas de mesmo corpo e gatilhos
diferentes não saem iguais.

**Revisar de novo quando o pool passar de ~80**, ou quando a DSL ganhar
primitivas novas — nesse momento entram tipos de efeito que o sigilo ainda não
sabe desenhar. Gatilho com alvo e gatilho de morte são os próximos candidatos,
e o segundo vai precisar de um anel que diga *quando*, não só *se*.
