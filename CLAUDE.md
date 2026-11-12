# Vértice

Jogo de duelo de comandantes com campanha de escola. Motor de regras em Rust,
cliente React + PixiJS, servidor autoritativo. IP própria — nada de franquia.

Dev solo. Isso está escrito aqui porque muda toda decisão: prefira sempre a
opção que um único dev consegue manter por três anos, não a mais elegante.

---

## 1. Orientação em 60 segundos

O jogo é um card game de 100 cartas singleton (formato comandante) com
overworld de escola: calendário, aulas, vínculo com NPC, duas reputações
(ficha e fama), e "ecos" — assinaturas de lendas que o aluno sintoniza como
comandante. Lore completa em `docs/biblia-de-historia.md`, decisões técnicas
em `docs/arquitetura.md`.

**A tese técnica:** existe um motor puro, sem I/O, determinístico, que
compila nativo no servidor e para WASM no cliente. Todo o resto do sistema
existe para proteger esse motor.

---

## 2. Regras invioláveis

Estas não são sugestões. Violá-las quebra o jogo de formas que só aparecem
semanas depois.

1. **`vertice-core` não tem I/O e não tem não-determinismo.** Sem
   `thread_rng`, `SystemTime`, `Instant::now`, `HashMap`, `HashSet`. RNG e
   tempo entram por parâmetro. Guarda automatizado:
   `./vertice-core/scripts/check-determinism.sh`.
2. **`legal_actions()` é o único validador de legalidade.**
   `apply()` aceita exatamente o que ela devolve. Não crie um segundo caminho
   de validação — é assim que cliente e servidor passam a discordar.
3. **O cliente nunca recebe `GameState`.** Só `view(state, viewer)`. Nem para
   debugar. Toda informação oculta (mão alheia, ordem da biblioteca) é
   fronteira de segurança, não detalhe de UI.
4. **Servidor é autoritativo.** O cliente prevê e anima; quando diverge, o
   servidor vence e o cliente ressincroniza.
5. **Carta publicada não é editada.** Mude o conjunto, nunca a carta.
   Replay de uma partida antiga depende de a carta ter o mesmo comportamento
   para sempre.
6. **Zero nome de franquia existente.** Nenhum personagem, carta, plano ou
   termo de Magic (ou de qualquer outra IP) entra no projeto — nem como
   referência em documento público. Ver `docs/arquitetura.md` §13.
7. **Corte antes de adicionar.** Se uma mecânica do Magic original não está
   implementada, ela não está faltando: ela foi cortada de propósito
   (`docs/arquitetura.md` §4).

---

## 3. Mapa de arquivos — quem é dono de quê

| Caminho | Contém | Dono (agente) |
|---|---|---|
| `vertice-core/src/engine.rs` | transições: `new_game`, `legal_actions`, `apply` | `engine-rules` |
| `vertice-core/src/state.rs` | `GameState`, zonas, `Object`, fingerprint | `engine-rules` |
| `vertice-core/src/action.rs` | `Action` (Copy, serializável) | `engine-rules` |
| `vertice-core/src/rng.rs` | RNG determinístico com estado serializável | `determinism-guard` |
| `vertice-core/src/view.rs` | redação por jogador — fronteira de segurança | `redaction-auditor` |
| `vertice-core/src/record.rs` | `MatchRecord`, replay | `determinism-guard` |
| `vertice-core/src/cards.rs` | catálogo (hoje `&'static`, vira dado no M1) | `card-content` |
| `vertice-core/tests/replay.rs` | determinismo e reprodução | `determinism-guard` |
| `vertice-core/tests/redaction.rs` | garantia de não vazamento | `redaction-auditor` |
| `vertice-server/` *(M1)* | Axum, ator tokio por partida, reconexão | `server-authority` |
| `vertice-client/` *(M2)* | React (chrome) + PixiJS (cenário e mesa) | `client-overworld` |
| `docs/biblia-de-historia.md` | cânone: casas, atos, NPCs | `lore-guardian` |
| `docs/arquitetura.md` | decisões técnicas e o porquê | `lore-guardian` (cuida do porquê) |

---

## 4. Índice de símbolos — comece por aqui

Agente novo, comece por esta tabela em vez de varrer o repositório.

| Preciso… | Vá para |
|---|---|
| criar uma partida | `engine.rs::new_game` |
| saber o que é legal agora | `engine.rs::legal_actions` |
| aplicar uma ação | `engine.rs::apply` |
| entender o formato do estado | `state.rs::GameState` |
| ver o que o cliente enxerga | `view.rs::view` |
| reproduzir uma partida | `record.rs::replay` |
| checar igualdade de estado | `state.rs::fingerprint` |
| ver o catálogo de cartas | `cards.rs::CATALOG` |

Fluxo normal de um turno, em ordem:
`view → legal_actions → apply → (muta GameState) → view`.

Se um bug não aparece nesse fluxo, ele está em outra camada.

---

## 5. Comandos

```bash
cd vertice-core
cargo test                      # 10 testes: replay, redação, legalidade
./scripts/check-determinism.sh  # falha se o core ganhar não-determinismo
cargo clippy --all-targets -- -D warnings
```

**Portão de qualidade:** nenhuma mudança no core é concluída sem os três
comandos acima verdes. Não existe "depois eu rodo".

---

## 6. Agentes

Sete agentes, repartidos por **camada**, não por tarefa. Camada é fronteira
estável; tarefa muda toda semana e geraria sobreposição.

Ativos agora:

| Agente | Papel | Escreve? |
|---|---|---|
| `engine-rules` | conserta regras no motor | sim |
| `determinism-guard` | audita replay e determinismo | **não** |
| `redaction-auditor` | audita vazamento de informação oculta | **não** |
| `card-content` | catálogo, DSL, curva de custo, texto de carta | sim |
| `lore-guardian` | cânone, consistência narrativa, segurança de IP | sim (docs) |

A partir do marco indicado:

| Agente | Papel | Quando |
|---|---|---|
| `server-authority` | autoridade, atores, reconexão, persistência | M1 |
| `client-overworld` | React, Pixi, calendário, transição de cena | M2 |

**Por que dois auditores são read-only:** quem conserta tem incentivo para
achar que consertou. O auditor que não pode editar é obrigado a provar com
teste. É a única forma de o dev solo ter revisão de código.

---

## 7. Roteamento por sintoma

Quando um agente recebe um bug, primeiro classifique pelo sintoma. Esta
tabela existe para impedir que todo mundo mexa em `engine.rs`.

| Sintoma | Agente | Primeiro comando |
|---|---|---|
| replay diverge, `fingerprint` muda entre execuções | `determinism-guard` | `check-determinism.sh`, depois `tests/replay.rs` |
| mesmo seed gera partidas diferentes | `determinism-guard` | `tests/replay.rs::mesmo_seed_mesmo_estado` |
| mão ou biblioteca do oponente aparece na view | `redaction-auditor` | `tests/redaction.rs` |
| ação legal recusada, ou ilegal aceita | `engine-rules` | `legal_actions` do estado em questão |
| estado errado depois de uma ação (vida, dano, zona) | `engine-rules` | reproduza com `MatchRecord` até a ação |
| criatura não morre / morre cedo | `engine-rules` | `engine.rs::cleanup` |
| partida não retoma após queda de conexão | `server-authority` | log da partida + `view` reidratada |
| tela, input, animação, transição | `client-overworld` | reprodutor de rede com partida gravada |
| carta com custo fora da curva, texto ambíguo | `card-content` | validador em `vertice-tools` |
| nome suspeito de IP, conflito com o cânone | `lore-guardian` | `docs/biblia-de-historia.md` |

---

## 8. Protocolo de correção

Vale para todo agente que escreve código. Cinco passos, nesta ordem.

1. **Reproduza.** Antes de tocar em qualquer arquivo, obtenha um estado ou
   teste que falhe. Bug que não reproduz não foi entendido.
2. **Localize pela camada.** Use o índice de símbolos e o roteamento. Se o
   bug atravessa duas camadas, resolva a de baixo primeiro — o sintoma de
   cima costuma sumir.
3. **Escreva o teste que falha.** No core, sempre em
   `tests/replay.rs` ou `tests/redaction.rs`. O fix só é aceito se o teste
   passar antes de você considerá-lo concluído.
4. **Corrija no menor escopo possível.** Uma camada por vez. Se o fix exigir
   mudar contrato (assinatura pública, formato de estado, protocolo), pare e
   registre a decisão em `docs/arquitetura.md` antes de continuar.
5. **Prove.** `cargo test` + `check-determinism.sh` + `clippy`. Cole a saída
   no relatório — "deve funcionar" não é prova.

### Relatório de agente

Formato curto, sempre:

```
SINTOMA:    o que o usuário viu
CAUSA:      arquivo:linha e o mecanismo
FIX:        o que mudou e por que nesse escopo
PROVA:      comando + resultado
RISCO:      o que pode ter quebrado junto
```

---

## 9. Convenções

**Nomenclatura.** `Vértice` com acento em texto para humano; `vertice` sem
acento em identificador (crate, pasta, binário, slug, domínio). Acento em
identificador quebra CI, Docker, path de Windows e nome de pacote.

**Rust.** Identificadores em inglês, comentários de decisão em português
quando explicam *por que*, não *o que*. Todo comentário que justifica uma
escolha não óbvia vale mais que dez que descrevem código.

**Determinismo é hábito, não teste.** Ao escrever qualquer laço que percorre
coleção, pergunte: a ordem é estável? Se a resposta for "depende", use
`BTreeMap`/`BTreeSet`/`Vec`.

**Texto de sinalização no jogo:** português do Brasil, registro de escola
técnica, sem gíria datada.

---

## 10. Não faça

- Não abra uma camada nova (servidor, cliente, launcher) antes do marco da
  camada anterior estar fechado.
- Não troque o conjunto de regras simplificado pelo completo do Magic.
  O corte é o que torna o projeto possível (`docs/arquitetura.md` §4).
- Não otimize prematuramente. O alvo é 60 fps num duelo 1v1, não escalar
  para dez mil partidas.
- Não adicione dependência no `vertice-core`. Zero é uma feature.
- Não escreva capítulo de história sobre regra que ainda vai mudar.
- Não edite carta publicada. Nunca.

---

## 11. Estado atual

**M0 — feito.** `vertice-core` com estado, ações, resolução, redação e
replay. 10 testes verdes. Guarda de determinismo funcionando (testado contra
violação plantada).

**M1 — próximo.** Zona de pilha real; carta como dado versionado com
validador (`vertice-tools`); `vertice-server` com um ator tokio por partida e
reconexão. Critério de saída: dois navegadores jogam, um cai, volta, e a
partida continua de onde estava.

**Pendências registradas:** `CardId` precisa ser estável e content-addressed
antes de o catálogo sair de `&'static` — reordenar o arquivo de cartas quebra
replay silenciosamente. `vertice-core/Cargo.lock` deve ser commitado quando o
primeiro binário (servidor) entrar no workspace.
