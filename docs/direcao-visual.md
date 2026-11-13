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
| marca central | efeito — ponto (nenhum), barras (dano), círculos (compra), traço (pump) |
| anel pontilhado | palavra-chave, um anel por palavra |

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

O sistema foi julgado com 13 cartas. **Revisar quando o pool passar de ~60**,
olhando o acervo inteiro de uma vez: se fichas diferentes começarem a se
parecer demais, o ajuste é na gramática do sigilo (mais um eixo), não em
desenhar carta por carta.
