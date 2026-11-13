# Roadmap de execução

Documento de trabalho. Diferente de `arquitetura.md` (que registra o *porquê*
das decisões), este aqui existe para ser **executado em fatias** — uma por
sessão de trabalho, com critério de saída verificável.

## Como usar

1. Escolha a próxima fatia não concluída. Não pule para outra camada.
2. Leia só a fatia. Não leia o roadmap inteiro antes de codar.
3. Rode os portões antes de começar (baseline verde) e no fim.
4. Commit por fatia, com o id da fatia no assunto: `M1.2: carta como dado`.
5. Se a fatia não fechar em uma sessão, **pare no ponto verde** e anote onde
   parou. Fatia pela metade não vira duas.

**Portão, para toda fatia de código:**

```bash
cd vertice-core && cargo test && ./scripts/check-determinism.sh
cargo clippy --all-targets -- -D warnings
```

Fatia que não passa no portão não está concluída, mesmo que "funcione na mão".

---

## Visão geral

| Marco | Conteúdo | Estado |
|---|---|---|
| M0 | motor determinístico, redação, replay | **feito** |
| M1 | pilha real, carta como dado, servidor autoritativo, cliente feio | **feito** |
| M2 | Pixi, chibi, transição de cena, bot heurístico, duelo offline | em andamento (falta chibi e transição do overworld) |
| M3 | quatro jogadores, espectador, replay público | pendente |
| M4 | calendário, casas, ecos, ato I, contas | pendente |
| M5 | Tauri, Steam, cosmético, loja | pendente |

Critérios de saída de M2–M5 estão em `arquitetura.md` §10. Não detalhe demais
o que está longe: plano de três anos escrito hoje é ficção.

---

## M1 — fatias

### M1.1 Pilha real

**Objetivo.** A mágica sai da mão para a pilha e só vai ao cemitério quando
resolve. Alvo que deixa o campo antes da resolução faz a mágica **fizzlar**:
não faz nada, mas continua tendo sido conjurada e paga.

**Arquivos.** `engine.rs` (`cast`, `pass`, `resolve`), `state.rs`
(fingerprint inclui a pilha), `record.rs` (mãos iniciais no registro),
`tests/stack.rs` (novo).

**Decisão que não muda.** Continua uma janela única de reação, pilha máxima de
2. Não é o stack do Magic e não vai virar.

**Saída.**
- Mágica conjurada não aparece no cemitério até resolver.
- Alvo removido antes da resolução ⇒ efeito não acontece, carta vai ao
  cemitério, vida não muda.
- `MatchRecord` com mãos forçadas reproduz bit a bit.

**Estado: feito.**

### M1.2 Carta como dado

**Objetivo.** Catálogo sai de `cards.rs` e vira dado versionado, com id
estável e validador.

**Decisão central — id estável.** `CardId` não pode ser posição em array.
Criar `cards/registry.toml`: `slug -> id`, **append-only**. O validador
recusa remoção de slug, remoção de id e reuso de id. Isso mata a armadilha de
reordenar o arquivo e quebrar todo replay existente.

**Forma.** Cartas em TOML, uma por arquivo em `cards/set-m1/`. `vertice-tools`
tem dois comandos: `validate` (custo na curva, palavra-chave conhecida, id
único e estável) e `generate` (emite `src/generated/cards.rs`). O motor
continua com zero dependências — o TOML é lido em tempo de build/validação,
não em tempo de jogo.

**Saída.**
- `cargo run -p vertice-tools -- validate` verde.
- Regenerar o código a partir do dado não muda nenhum `CardId`.
- Inserir carta no meio do conjunto é **recusado** pelo validador, não
  apenas desencorajado por convenção.
- Todos os testes de replay continuam verdes.

**Estado: feito.** `next_id` foi acrescentado ao registro porque derivar o
próximo id do lock deixava apagar carta passar calado — descoberto por
auditoria, não por teste.

### M1.3 Servidor autoritativo 1v1

**Objetivo.** Duas conexões jogam a mesma partida; o servidor é a verdade.

**Decisão revista.** Postgres sai do M1. Persistência de partida é
`MatchRecord` em SQLite (ou arquivo JSON). Postgres só quando existir conta
de verdade, no M4. Motivo: um dev solo não mantém banco que não usa.

**Forma.** Axum + WebSocket. Uma task tokio por partida, dona do `GameState`,
recebendo `Action` por `mpsc`. Protocolo versionado e com índice de mensagem.
Cada conexão recebe a view **do seu próprio jogador**.

**Saída — automatizada, não manual.** Teste de integração com dois clientes
WebSocket headless jogando até o fim. Um deles derruba a conexão no meio,
reconecta, e a partida continua exatamente de onde estava (mesmo
`fingerprint`). Esse teste é o marco. "Abri dois navegadores" não é prova.

**Estado: feito.** Protocolo versionado com `seq`, ator por partida, reconexão
e persistência em arquivo por partida. Entrou junto o **fim de partida** — o
motor não tinha condição de vitória, então "jogar até o fim" não significava
nada; hoje vida <= 0 encerra, checado ao fim da ação, sem empate.

### M1.4 Cliente mínimo, feio de propósito

**Objetivo.** Provar o protocolo na interface, sem investir em arte.

**Forma.** React + Vite, sem Pixi. Botões gerados a partir de
`legal_actions`. Sem animação, sem chibi, sem transição.

**Saída.** Partida completa em dois navegadores, com queda e retomada. Se
algo do protocolo doer, dói aqui — quando ainda é barato mudar.

**Estado: feito.** `POST /matches`, binário do servidor e cliente React
mínimo, verificado em navegador e coberto por teste de renderização com
visões fixas. O teste headless prova a partida até a vitória; o de
renderização prova a tela — cada tipo de ação vira botão, o clique manda a
ação exata, e fim de partida remove os botões.

---

## Depois do M1

**M2** é onde o jogo ganha cara: Pixi, mapa, chibi, transição de cena, bot
heurístico para sustentar a campanha, duelo offline via WASM. É também onde o
orçamento de arte começa a morder — decidir direção iconográfica antes de
encomendar qualquer ilustração.

**M3** e **M4** não devem ser planejados em detalhe agora. Quando o M2
fechar, este documento é reescrito.

---

## Brief pronto para colar

> Implemente a fatia **M1.2** de `docs/roadmap.md`, seguindo `CLAUDE.md`.
> Leia só a seção da fatia. Rode os portões antes de começar e no fim, e cole
> a saída no relatório. Não avance para a fatia seguinte. Se algo exigir
> decisão que não está escrita, pare e pergunte em vez de escolher.

Trocar o id da fatia é a única edição necessária. O resto é fixo — e é de
propósito: agente que recebe escopo aberto expande o escopo.
