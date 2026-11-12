# Vértice

Jogo de duelo de comandantes ambientado numa escola onde as leylines dos planos
se cruzam. Overworld com avatar chibi, calendário de aulas, vínculo com NPC,
duas reputações — e duelos em tela própria, com deck de 100 cartas singleton.

IP própria. Motor de regras próprio. Dev solo.

## Estrutura

```
docs/biblia-de-historia.md   cânone: mundo, casas, atos, elenco
docs/arquitetura.md          decisões técnicas e o porquê de cada uma
vertice-core/                motor de regras (Rust, determinístico, sem I/O)
CLAUDE.md                    convenções do projeto e repartição de agentes
```

## Começar

```bash
cd vertice-core
cargo test
./scripts/check-determinism.sh
```

## Princípios

O motor é puro e determinístico: dado (decks, seed, ações), a partida reproduz
bit a bit. Ele compila nativo no servidor e para WASM no cliente, então as
regras existem em um lugar só. O cliente nunca recebe o estado completo — só a
view redigida por jogador.

O conjunto de regras é deliberadamente menor que o de Magic: sem sistema de
camadas, sem efeitos de substituição, janela única de reação. O corte não é
preguiça, é o que torna o projeto viável para uma pessoa.

## Estado

M0 concluído (motor com replay e redação, 10 testes). M1 em andamento:
pilha real, carta como dado versionado, servidor autoritativo com reconexão.
