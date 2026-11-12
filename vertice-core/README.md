# vertice-core — M0

Motor de regras do **O Vértice**. Este é o marco zero: existe para provar que
o conjunto de regras é implementável, determinístico e não vaza informação
oculta — antes de existir arte, rede ou banco.

## Por que este crate é assim

| Regra | Motivo |
|---|---|
| Zero dependências | Árvore de dependência vazia é a forma mais barata de garantir determinismo |
| Sem `thread_rng`, `SystemTime`, `HashMap` | Cada um quebra replay. `scripts/check-determinism.sh` falha o build |
| `BTreeMap` no estado, `Vec` nas zonas | Ordem de iteração estável por construção |
| RNG e tempo por parâmetro | O mesmo código roda no servidor e em WASM no cliente |
| `view()` é a única saída de estado | Cliente que recebe estado completo é cliente que trapaceia |

## Comandos

```bash
cargo test                      # 10 testes: replay, redação, legalidade
./scripts/check-determinism.sh  # falha se o core ganhar não-determinismo
cargo clippy -- -D warnings
```

## Estrutura

```
src/ids.rs      PlayerId, CardId, ObjectId (instância, não carta)
src/rng.rs      xorshift64* com estado serializável — entra no GameState
src/cards.rs    catálogo &'static do M0 (vira dado versionado no M1)
src/state.rs    GameState, zonas, Object, fingerprint
src/action.rs   Action — valor imutável, Copy, serializável
src/engine.rs   new_game, legal_actions, apply. Único validador de legalidade
src/view.rs     redação por jogador. A fronteira de segurança
src/record.rs   MatchRecord + replay
tests/replay.rs     determinismo e reprodução bit a bit
tests/redaction.rs  garantia de que mão e biblioteca alheias não vazam
tests/stack.rs      pilha, fizzle e abertura roteirizada
```

## O contrato

```
cliente  ->  Action
servidor ->  view(state, viewer)     // nunca o estado
partida  =   (engine_version, card_set_version, seed, decks, actions[])
```

`apply()` só aceita o que `legal_actions()` devolve. Não existe segundo
caminho de validação, e não deve existir.

## Simplificações conscientes do motor

Todas reversíveis, todas documentadas no topo de `engine.rs`:

- Mana automática igual ao número de terrenos no início do turno; terrenos não viram.
- Sem etapa de bloqueio: atacar criatura é troca de dano direta.
- Pilha de no máximo 2 (uma mágica + uma resposta) — a "janela de reação única".
- Criaturas não usam a pilha; só mágicas abrem janela.
- Mágica ocupa a pilha e só vai ao cemitério ao resolver; alvo inválido faz fizzlar.

## O que este marco NÃO tem

Rede, banco, interface, bot, arte, quatro jogadores. Se alguma dessas coisas
aparecer aqui antes do M0 fechar, o escopo vazou.

## Próximo passo (M1)

1. Zona de pilha real, com resolução ordenada.
2. Carta como dado: DSL + validador em `vertice-tools`.
3. `vertice-server`: Axum, um ator tokio por partida, `view` redigida por
   conexão, reconexão com reidratação a partir do log.
4. Alvo: dois navegadores jogando, um cai, volta, e continua.
