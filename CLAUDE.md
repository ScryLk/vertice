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
| `vertice-core/src/cards.rs` | tipos da DSL (`CardDef`, `Keyword`, …) e `card(id)` | `card-content` |
| `vertice-core/src/generated/` | `CATALOG` compilado de `cards/`. **Gerado e commitado** | `card-content` |
| `vertice-core/tests/replay.rs` | determinismo e reprodução | `determinism-guard` |
| `vertice-core/tests/redaction.rs` | garantia de não vazamento | `redaction-auditor` |
| `vertice-core/tests/stack.rs` | pilha, fizzle e abertura roteirizada | `engine-rules` |
| `vertice-core/tests/outcome.rs` | fim de partida e desempate por prioridade | `engine-rules` |
| `vertice-core/tests/catalog.rs` | invariantes do catálogo gerado | `card-content` |
| `vertice-core/tests/golden.rs` | partida dourada com fingerprint fixo | `determinism-guard` |
| `cards/registry.toml` | `slug -> CardId` e `next_id`. Append-only | `card-content` |
| `cards/registry.lock` | hash de cada carta publicada. Gerado | `card-content` |
| `cards/set-*/` | as cartas em TOML | `card-content` |
| `vertice-tools/` | `validate` e `generate` do catálogo | `card-content` |
| `vertice-server/src/protocol/` | DTOs de rede; espelho em `vertice-client/src/protocol.ts` | `server-authority` |
| `vertice-server/src/actor.rs` | uma task por partida, dona do `GameState` | `server-authority` |
| `vertice-server/src/{http,registry,store}.rs` | WebSocket, `POST /matches`, persistência | `server-authority` |
| `vertice-client/` | React mínimo, sem Pixi (Pixi é M2) | `client-overworld` |
| `docs/biblia-de-historia.md` | cânone: casas, atos, NPCs | `lore-guardian` |
| `docs/arquitetura.md` | decisões técnicas e o porquê | `lore-guardian` (cuida do porquê) |

---

## 4. Índice de símbolos — comece por aqui

Agente novo, comece por esta tabela em vez de varrer o repositório.

| Preciso… | Vá para |
|---|---|
| criar uma partida | `engine.rs::new_game` |
| criar partida com mão fixa (teste, campanha) | `engine.rs::new_game_with_hands` |
| saber o que é legal agora | `engine.rs::legal_actions` |
| aplicar uma ação | `engine.rs::apply` |
| entender o formato do estado | `state.rs::GameState` |
| ver o que o cliente enxerga | `view.rs::view` |
| reproduzir uma partida | `record.rs::replay` |
| checar igualdade de estado | `state.rs::fingerprint` |
| ver o catálogo de cartas | `cards.rs::CATALOG` |
| saber se a partida acabou e quem venceu | `state.rs::GameState.winner` |
| validar ou gerar o catálogo | `cargo run -p vertice-tools -- validate` / `generate` |
| criar uma partida no servidor | `vertice-server/src/registry.rs::Registry::create` |
| saber a forma do protocolo | `vertice-server/src/protocol/` |
| subir o servidor | `vertice-server/src/main.rs` (`VERTICE_DATA`, `PORT`) |

Fluxo normal de um turno, em ordem:
`view → legal_actions → apply → (muta GameState) → view`.

Se um bug não aparece nesse fluxo, ele está em outra camada.

---

## 5. Comandos

```bash
cd vertice-core
cargo test                      # 27 testes: replay, redação, pilha, catálogo, desfecho
./scripts/check-determinism.sh  # falha se o core ganhar não-determinismo
cargo clippy --all-targets -- -D warnings
```

Da raiz, para o workspace inteiro (`vertice-core`, `vertice-tools`,
`vertice-server`):

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vertice-tools -- validate   # ids, curva, append-only, hash, geração em dia
npm --prefix vertice-client run build    # type-check + build do cliente
```

**Portão de qualidade:** nenhuma mudança no core é concluída sem `cargo test`,
`check-determinism.sh` e `clippy` verdes. Mudança em `cards/` acrescenta
`validate`; mudança no cliente acrescenta o build. Não existe "depois eu rodo".

---

## 6. Agentes

Sete agentes, repartidos por **camada**, não por tarefa. Camada é fronteira
estável; tarefa muda toda semana e geraria sobreposição.

Ativos agora (todos os sete, desde o M1.4):

| Agente | Papel | Escreve? |
|---|---|---|
| `engine-rules` | conserta regras no motor | sim |
| `determinism-guard` | audita replay e determinismo | **não** |
| `redaction-auditor` | audita vazamento de informação oculta | **não** |
| `card-content` | catálogo, DSL, curva de custo, texto de carta | sim |
| `lore-guardian` | cânone, consistência narrativa, segurança de IP | sim (docs) |
| `server-authority` | autoridade, atores, reconexão, persistência | sim |
| `client-overworld` | React agora; Pixi e transição de cena no M2 | sim |

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
| protocolo divergiu do motor (campo novo, ação que não chega) | `server-authority` | `tests/roundtrip.rs` e a forma do JSON fixada |
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
replay. Guarda de determinismo funcionando (testado contra violação plantada).

**M1.1 — feito.** Pilha real: a mágica ocupa a pilha e só vai ao cemitério ao
resolver; alvo que sai do campo antes da resolução faz a mágica fizzlar;
abertura roteirizada reproduz bit a bit.

**M1.2 — feito.** Carta como dado. `cards/registry.toml` é a única fonte de
`CardId`, com `next_id` vivendo fora do lock para que deleção não passe calada;
`cards/set-m1/` tem as 13 cartas em TOML; `vertice-tools` valida e gera
`vertice-core/src/generated/cards.rs`, que é **commitado**. Inserir carta no
meio, apagar carta, reusar id e editar carta publicada são recusados pelo
validador, provado por teste.

**M1.3 — feito.** Servidor autoritativo: protocolo versionado com `seq`, uma
task por partida dona do `GameState`, reconexão no mesmo `fingerprint`, e
persistência em arquivo por partida. Fim de partida: vida <= 0 encerra,
checado ao fim da ação, sem empate — prioridade desempata.

**M1.4 — entregue, com uma ressalva.** `POST /matches`, binário do servidor e
cliente React mínimo, verificado em navegador: criar partida, entrar nos dois
assentos, agir, recarregar e reconectar, e a mão do oponente chegando como
contagem. **Falta** jogar uma partida inteira clicando nos dois assentos — a
partida completa até a vitória só está provada em WebSocket headless.

**Próximo: fechar a ressalva do M1.4** e então o M2 (Pixi, chibi, transição de
cena, bot heurístico, duelo offline). O roteiro está em `docs/roadmap.md`;
siga por ele em vez de reabrir o plano.

**Pendências registradas:**
- `CARD_SET_VERSION` continua `"m0.1"` e é etiqueta, não portão: `replay()`
  nunca o compara. Vira portão quando houver serialização, comparando o hash
  do conjunto gravado no registro.
- Não existe CI (`.github`): os portões são manuais.
- Matchmaking não existe: `POST /matches` cria a mesa e basta para o M1.
