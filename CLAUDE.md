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
| `vertice-core/tests/trigger.rs` | gatilho de entrada: quando dispara e quando não | `engine-rules` |
| `vertice-core/tests/outcome.rs` | fim de partida e desempate por prioridade | `engine-rules` |
| `vertice-core/tests/catalog.rs` | invariantes do catálogo gerado | `card-content` |
| `vertice-core/tests/golden.rs` | partida dourada com fingerprint fixo | `determinism-guard` |
| `cards/registry.toml` | `slug -> CardId` e `next_id`. Append-only | `card-content` |
| `cards/registry.lock` | hash de cada carta publicada. Gerado | `card-content` |
| `cards/set-*/` | as cartas em TOML | `card-content` |
| `vertice-tools/` | `validate` e `generate` do catálogo | `card-content` |
| `vertice-protocol/` | DTOs de rede — servidor e WASM falam a mesma forma | `server-authority` |
| `vertice-server/src/` | `protocol` reexportado de `vertice-protocol`; espelho em `vertice-client/src/protocol.ts` | `server-authority` |
| `vertice-server/src/actor.rs` | uma task por partida, dona do `GameState` | `server-authority` |
| `vertice-server/src/{http,registry,store}.rs` | WebSocket, `POST /matches`, persistência | `server-authority` |
| `vertice-client/src/Ficha.tsx` | a carta como verbete; o sigilo é gerado do dado | `client-overworld` |
| `vertice-client/src/{App,Duel,Catalogo}.tsx` | lobby, mesa e acervo | `client-overworld` |
| `vertice-client/src/acervo.ts` | catálogo por `CardId` vindo do WASM; é o que deixa a mão mostrar o texto da carta | `client-overworld` |
| `vertice-client/src/cenario/` | Pixi: a mesa e as leylines, tudo procedural | `client-overworld` |
| `vertice-bot/src/eval.rs` | pesos da avaliação de tabuleiro | `engine-rules` |
| `vertice-bot/src/lib.rs` | `choose_action` e níveis de dificuldade | `engine-rules` |
| `vertice-wasm/src/lib.rs` | duelo offline no navegador (`Duel` puro + binding) | `client-overworld` |
| `docs/biblia-de-historia.md` | cânone: casas, atos, NPCs | `lore-guardian` |
| `docs/arquitetura.md` | decisões técnicas e o porquê | `lore-guardian` (cuida do porquê) |
| `docs/direcao-visual.md` | o que o jogo se parece, e o que isso custa | `lore-guardian` |

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
| fazer o bot escolher uma jogada | `vertice-bot/src/lib.rs::choose_action` |
| criar partida contra o bot | `vertice-server/src/registry.rs::Registry::create_vs_bot` |
| ajustar o que o bot valoriza | `vertice-bot/src/eval.rs` |
| ver o acervo na linguagem visual | `?cartas` no cliente, ou `src/Ficha.tsx` |
| mexer no cenário do duelo | `vertice-client/src/cenario/mesa.ts` |
| mudar a direção visual | `docs/direcao-visual.md` antes de qualquer código |
| jogar sem servidor, no navegador | `vertice-wasm/src/lib.rs::Duel` |

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
`vertice-server`, `vertice-bot`, `vertice-protocol`, `vertice-wasm`):

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vertice-tools -- validate   # ids, curva, append-only, hash, geração em dia
npm --prefix vertice-client test         # renderização do cliente (Vitest)
npm --prefix vertice-client run build    # wasm + type-check + build do cliente
```

**Portão de qualidade:** nenhuma mudança no core é concluída sem `cargo test`,
`check-determinism.sh` e `clippy` verdes. Mudança em `cards/` acrescenta
`validate`; mudança no cliente acrescenta `test` e o build. Não existe "depois eu rodo".

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
| bot escolhe jogada estranha ou joga mal | `engine-rules` | `vertice-bot/tests/bot.rs` (placar espelhado) |
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

**M1.4 — feito.** `POST /matches`, binário do servidor e cliente React mínimo.
Verificado em navegador — criar partida, entrar nos dois assentos, agir,
recarregar e reconectar, e a mão do oponente chegando como contagem — e
coberto por teste de renderização com visões fixas: cada tipo de ação vira
botão, o clique manda a ação exata, e fim de partida remove os botões. O teste
headless prova a partida; o de renderização prova a tela.

**M2 — em andamento, pelo bot.** `vertice-bot` escolhe jogada a um passo:
aplica cada ação legal num clone e avalia a posição resultante. Níveis
`Calouro` e `Formando` diferem só na frequência de erro deliberado. Medido com
placar espelhado (duas orientações na mesma seed, para o viés de assento
cancelar): formando 22×2 no calouro e 24×0 no aleatório, em 24 partidas cada.

O bot **joga no servidor**: `POST /matches` com `level` cria a partida com o
assento 1 controlado por ele, e o ator da partida o faz jogar enquanto for a
vez dele — o cliente não pede nada por ele. Verificado em navegador: "Jogar
contra o formando" e cinco turnos depois o humano estava em 0 de vida.
Partida contra bot **não é gravada** (ver `arquitetura.md` §5).

**Duelo offline por WASM — feito.** `vertice-wasm` compila o motor para o
navegador e expõe a mesma forma de JSON que o servidor manda, então a tela não
sabe a diferença entre online e offline. Os DTOs saíram de `vertice-server`
para `vertice-protocol`, para o WASM falar a mesma forma sem arrastar axum.
Offline **não tem autoridade nenhuma** — é modo de treino e de campanha; quem
decide partida valendo continua sendo o servidor. O `.wasm` é gerado, não
commitado: `predev` e `prebuild` o reconstroem (1,6s incremental), então ele
nunca fica velho.

**Direção visual — decidida.** Verbete em tudo, sem cor: a carta é uma ficha
do Arquivo, e o reconhecimento vem de um sigilo gerado do dado (moldura pelo
tipo, traços pelo custo, marca pelo efeito), mais uma marca de catalogação
derivada do `CardId` para desempatar cartas de perfil igual. Uma linguagem, um
track de produção, custo marginal de carta nova igual a zero. Decisão, riscos
e gatilho de revisão em `docs/direcao-visual.md`; o acervo inteiro é visível em
`?cartas`.

**Pixi — feito, em camada separada.** O cenário do duelo é Pixi atrás do
React: pedra escura e leylines convergindo, **tudo procedural, nenhum asset**.
A divisão é a da `arquitetura.md` §1 — Pixi desenha o que não tem texto, React
desenha o que tem — e por isso as fichas continuam no DOM, onde layout e
tipografia já funcionam. Sem WebGL o cenário não aparece e a mesa segue
jogável, porque nada nele carrega informação. A entrada do duelo (as leylines
abrindo em ~1,1s) é o encaixe onde a transição do overworld vai plugar quando
o overworld existir, no M4.

**Pool cresceu para 50.** 37 cartas novas em `cards/set-m2/`, ids 13..49,
`next_id = 50`, `CARD_SET_VERSION = "m2.1"`. O validador aceitou todas de
primeira — a curva estava certa antes de o dado existir.

**O teto do pool não é de conteúdo, é de DSL.** Com três palavras-chave e três
efeitos, ~50 cartas é onde a distinção acaba: daqui para cima o que se produz é
duplicata com outro nome. Chegar às 200–250 que a `arquitetura.md` §6 pede
exige **primitivas novas** — destruir, criar ficha, modificar característica —,
e isso é trabalho de motor, não de catálogo.

**M2.1 — gatilho de entrada, e o pool em 66.** A primeira primitiva de motor
desde o M0: `CardDef.on_enter` dispara quando a criatura entra, mais o efeito
`gain_life`. 16 cartas novas em `cards/set-m2/`, ids 50..65, `next_id = 66`,
`CARD_SET_VERSION = "m2.2"`, `ENGINE_VERSION = "0.3.0"`. O estado ganhou
**nenhum** campo — o gatilho é dado da carta e resolve dentro da conjuração.

O corte é o alvo: gatilho com alvo exigiria que o gatilho usasse a pilha, e o
validador recusa a carta antes de o motor poder ignorá-la. A escolha veio de
medida, não de gosto: num corpus externo de ~34 mil cartas, 43% têm habilidade
disparada, 29% ativada, e o motor tinha zero de ambas — enquanto crescer de 3
para 20 palavras-chave movia o alcance de 1,4% para 2,2%. **Palavra-chave é a
alavanca mais fraca que existe.**

A curva de custo também foi recalibrada: a base da criatura virou
`⌈(P+T)/2⌉` e o `+1` fixo por palavra-chave virou peso por palavra (`arquitetura.md` §6).

**M2.2 — gatilho de morte, e o pool em 80.** `CardDef.on_die` e o efeito
`lose_life`. 14 cartas novas, ids 66..79, `next_id = 80`,
`CARD_SET_VERSION = "m2.3"`, `ENGINE_VERSION = "0.4.0"`. Mesmo modelo do M2.1 e
mesmo corte — sem alvo, resolve na hora —, agora dentro do `cleanup` e na ordem
do campo. **É o que dá sentido a trocar criatura**: sem gatilho de morte, atacar
em troca é sempre perder, e o duelo vira corrida de dano.

`on_die` vai **rotulado** no `content_hash` e `on_enter` não, e a assimetria é
dívida herdada de propósito: o slot de `on_enter` está no formato desde a
publicação e não pode ganhar rótulo sem reescrever o lock das cartas que já o
usam. Sem o rótulo em `on_die`, "ao entrar, compre 1" e "ao morrer, compre 1"
produziriam a mesma string canônica — há teste e mutação provando.

**M2.3 — gatilho de entrada com alvo, e o pool em 92.** `on_enter` passou a
aceitar efeito que pede alvo. **O alvo vem na própria conjuração**:
`Action::Cast { target: Some }` para criatura, o mesmo campo que a mágica já
usava e que para criatura era sempre `None`. Consequência de projeto: nenhuma
ação nova, nenhum campo novo no protocolo, nenhuma interação nova na tela — e a
escolha acontece antes de o gatilho disparar, que é onde ela acontece. 12 cartas
novas, ids 80..91, `next_id = 92`, `CARD_SET_VERSION = "m2.4"`,
`ENGINE_VERSION = "0.5.0"`.

`on_die` com alvo **continua cortado**, e agora por um motivo sozinho: a criatura
morre pela ação de outra pessoa, e não existe ação do controlador para carregar
a escolha. Abrir isso é gatilho na pilha — a fatia que falta.

**M2.4 — destruir, e o pool em 102.** `Effect::Destroy`, o primeiro efeito cujo
resultado não é um número. 10 cartas novas, ids 92..101, `next_id = 102`,
`CARD_SET_VERSION = "m2.5"`, `ENGINE_VERSION = "0.6.0"`. Alvo é **só criatura**:
terreno não entra em `effect_targets`, porque oferecer um botão que o motor
recusa é pior que não oferecer.

**Destruir mata pelo mesmo caminho do dano letal.** `bury` saiu de dentro de
`cleanup` e virou função compartilhada — se destruir tivesse caminho próprio,
`on_die` funcionaria num e não no outro, e o defeito só apareceria quando
alguém destruísse uma criatura com gatilho e estranhasse o silêncio. Há teste e
mutação cobrindo exatamente isso.

Preço medido: 3 de mana, mediana de 117 mágicas de uma linha só que destroem
criatura. Não escala como o dano porque não tem magnitude.

O baralho provisório cicla o catálogo, então crescer o pool muda o placar do
bot junto: com 102 cartas, o calouro fez 17×7 contra o aleatório e a barra do
teste era 18. A barra desceu para 16 com a medição registrada no próprio teste.
**A lição:** aquele número é do bot *mais* o baralho provisório, não do bot.

**A mão do duelo passou a mostrar o que a carta faz.** A `PlayerView` carrega só
o id da carta, então a mão mostrava nome e corpo, e com gatilho com alvo o
jogador escolhia entre vários botões sem saber o que cada um fazia. Agora
`src/acervo.ts` indexa o catálogo do WASM por id e o `Duel` usa `resumoDaCarta`
(as mesmas frases da ficha) para escrever o efeito sob a carta. O rótulo do alvo
também deixou de ser `objeto 30` e virou o nome. Zero de protocolo: definição de
carta é dado público e não precisa viajar na view. O módulo chama-se `acervo` e
não `catalogo` porque `Catalogo.tsx` já existe e o macOS não diferencia
maiúscula de minúscula.

Falta do M2: **chibi e a transição a partir do overworld** — as duas dependem
de arte que não é procedural, e a segunda depende de uma cena que só existe no
M4. Ilustração de comandante e transição de cena continuam sendo o orçamento
de arte reservado (`arquitetura.md` §9).

**Próximo:** o resto do M2. O roteiro está em `docs/roadmap.md`; siga por ele
em vez de reabrir o plano.

**Pendências registradas:**
- `CARD_SET_VERSION` continua `"m0.1"` e é etiqueta, não portão: `replay()`
  nunca o compara. Vira portão quando houver serialização, comparando o hash
  do conjunto gravado no registro.
- Não existe CI (`.github`): os portões são manuais.
- Matchmaking não existe: `POST /matches` cria a mesa e basta para o M1.
