# O VÉRTICE
### Arquitetura técnica e plano de execução — dev solo, PC primário

---

## 1. Veredito

Sua stack está certa — mas por um motivo diferente do que você pensa. Não é a melhor stack objetivamente. É a melhor stack **para você**, porque você já é dev web e não tem folga pra aprender um motor de jogo *e* construir um motor de regras no mesmo projeto. Dev solo não tem orçamento de aprendizado.

A decisão mais cara do projeto não é React, nem Electron, nem Rust. É o motor de regras. Ele come 60–70% da engenharia e define se você entrega em dois anos ou em seis. Todo o resto desta arquitetura existe pra proteger o motor.

### As cinco decisões que valem

1. **Motor em crate Rust puro, sem I/O, determinístico, compilado nativo pro servidor e pra WASM no cliente.** É o único jeito de ter regras num lugar só, duelo offline e interface instantânea.
2. **Sem Electron.** Desenvolva no browser, empacote com Tauri só quando a Steam entrar na conversa. Não construa launcher na v1.
3. **React só pro chrome, PixiJS pro cenário.** Duas cenas separadas, com o módulo de duelo carregado sob demanda.
4. **Servidor autoritativo com estado redigido por jogador.** Zona oculta nunca chega no cliente. Isso não é segurança, é arquitetura — contamina o motor inteiro.
5. **Corte as regras do Magic.** O conjunto de regras é o maior alavancamento de escopo que você tem, e você tem IP própria pra cortar.

---

## 2. Mapa de módulos

| Módulo | Stack | Papel |
|---|---|---|
| `vertice-core` | Rust, no_std-friendly | Estado, ações, resolução, redação, RNG injetado. Zero I/O. |
| `vertice-cards` | Rust + dados | Definições de carta versionadas, compiladas a partir de DSL. |
| `vertice-bot` | Rust | IA heurística com níveis de dificuldade. Depende só do core. |
| `vertice-server` | Axum + tokio + Postgres | Gateway, matchmaking, sessões, persistência, autoridade. |
| `vertice-client` | React + Vite + PixiJS + TS | Overworld, duelo, menus, deck builder. |
| `vertice-wasm` | wasm-bindgen | Binding do core pro cliente (offline e preview local). |
| `vertice-tools` | Rust CLI | Validação de carta, geração de índice, testes de replay. |
| `launcher` | Tauri | Só na fase 5. Empacotamento e update. |

Uma regra dura: **`vertice-core` não pode importar `chrono`, `rand`, `reqwest`, nem nada com tempo ambiente.** RNG entra como parâmetro. Tempo entra como parâmetro. Ordenação de mapa usa `BTreeMap`, nunca `HashMap` — ordem de iteração de `HashMap` é não-determinística e vai te quebrar o replay de um jeito que você vai levar três dias pra achar. Coloque isso no CI: um `grep` que falha o build se aparecer `thread_rng`, `SystemTime` ou `HashMap` no core.

---

## 3. O contrato motor ↔ cliente

O motor é uma máquina de estados pura. O cliente manda **ações**, o servidor responde com **views**.

```
Ação do jogador  ->  { tipo, payload, match_id, seq }
View do servidor ->  { estado visível àquele jogador, ações legais, log desde seq }
```

Três propriedades que precisam ser verdade desde o dia um:

**Redação por espectador.** A view é gerada *para um jogador específico*. Função `view(state, player_id) -> VisibleState`. Mão dos outros é contagem, não conteúdo. Baralho é contagem e ordem oculta. O mesmo vale pra replay e espectador — funções de redação diferentes, mesma fonte.

**Determinismo.** Dado (decklists, seed, lista de ações), o motor produz exatamente a mesma partida. É o que permite replay, espectador, testes e reconciliação. Sem isso você não tem jogo competitivo.

**Autoridade.** O cliente nunca decide. Ele pode rodar o motor em WASM pra prever e animar, mas a verdade é do servidor. Se divergir, o servidor vence e o cliente resincroniza.

**Entrada hostil morre na porta.** `CardId` chega de fora — baralho do cliente, `MatchRecord` lido do banco. `new_game` recusa com `BadDeck` qualquer id fora do catálogo. `card()` continua entrando em pânico com id desconhecido, e isso é deliberado: id inválido no meio da partida é bug, não erro de entrada. A checagem existe na porta justamente para que esse pânico deixe de ser alcançável por dado de fora — uma partida que explode derruba a task que a hospeda, não a requisição que trouxe o baralho ruim.

O log de ações é a unidade de verdade. Partida salva = `{versao_motor, versao_cartas, seed, jogadores, acoes[]}`. Nada mais precisa ser guardado pra reproduzir.

**O protocolo é escrito à mão, de propósito (M1.3).** `vertice-server` traduz os tipos do motor para `serde` num `match` exaustivo; o motor não ganha `serde`, nem `#[derive(Serialize)]`, nem nada. O preço é duplicar a forma em dois lugares. O pagamento é que o compilador quebra a tradução quando `Action` ou `PlayerView` ganham uma variante — ninguém descobre isso em produção. Toda mensagem leva `type`; as de jogo levam `seq`; o `Join` declara `protocol` e versão diferente é recusa imediata, não negociação. A `PlayerView` é **mão única** (carrega `&'static str` do catálogo, e do outro lado só existe texto), enquanto o `MatchRecord` é **reversível** — é ele que reconexão e replay rejogam.

**Versão de registro é dado, não constante.** `MatchRecord.engine_version` e `card_set_version` são `String`, não `&'static str`: um registro lido do banco carrega a versão de *quando foi jogado*. Com `&'static str` o campo só conseguia reportar a constante de compilação — era etiqueta, não dado.

---

## 4. O que cortar das regras

Esta é a seção mais importante do documento. Cada mecânica que você mantiver do Magic original é uma pilha de casos de borda que você implementa, testa e buga sozinho, pra sempre.

**Corte o sistema de camadas.** Magic tem sete camadas de efeito contínuo com dependência entre elas. Você não precisa. Fixe uma ordem única e simples: características base → modificadores aditivos → modificadores condicionais, resolvidos em ordem de entrada. Documente como lei e nunca abra exceção. Se uma carta não cabe, a carta muda.

**Corte efeitos de substituição.** São o segundo maior buraco de bugs. Se precisar, permita exatamente um tipo: "em vez de morrer, acontece X". Um. Nunca empilhável.

**Corte janelas de prioridade livres.** Você quer interação, não quer o stack do Magic. Uma janela de reação por turno, declarada, com ordem de resposta fixa, dá 80% da sensação e 20% da complexidade. Se a mesa precisar de mais, você adiciona depois — tirar é que é caro.

**Corte resolução ambígua.** Nada de "cada jogador escolhe simultaneamente". Toda escolha é sequencial, com dono claro. Ambiguidade é inferno de implementação e de rede.

**Palavras-chave são seu melhor conteúdo.** Uma palavra-chave bem definida gera dezenas de cartas quase de graça. Cartas com texto único são caríssimas. Meta: 70% do pool usa palavras-chave da lista fechada, 30% tem texto próprio.

**Comece 1v1.** Faça um duelo completo funcionar ponta a ponta antes de tocar em quatro jogadores. Quatro jogadores multiplica interface, rede, IA e casos de regra por três ou quatro. O modo de quatro é o objetivo, não o ponto de partida.

---

## 5. Servidor

**Um processo, um binário.** Dev solo não precisa de escala horizontal. Axum + tokio, um Postgres, tudo numa VPS. Redis só quando você tiver mais de um gateway — e provavelmente nunca.

**Uma task por partida.** Cada partida é um ator tokio que possui o estado e recebe mensagens por canal. Sem estado mutável compartilhado, sem lock. Uma partida travada não derruba o servidor, e você mata ela com timeout.

**Presença e fila** ficam em memória do processo, num mapa protegido. Com uma instância, é suficiente e é honesto.

**Autenticação mínima:** email e senha, hash com argon2, JWT curto. Não construa OAuth, não construa 2FA, não construa painel de admin antes de ter jogador.

**Reconexão é requisito, não enfeite.** Partida de comandante dura 40 minutos. Cliente cai. O servidor guarda a partida e reidrata o jogador com a view redigida e o log. Faça isso no M1, não no fim.

**O que existe hoje (M1.3, parcial).** Uma task por partida dona do `GameState`, um WebSocket por conexão, e reconexão que reidrata da memória: o estado mora no ator, não na conexão, então cair não significa nada. O servidor recusa ação fora de `legal_actions` e índice atrasado sem mexer no estado. Provado por teste com dois clientes headless, um deles derrubado no meio, comparando `seq`, view e `fingerprint`. **Não existe:** persistência (reiniciar o processo perde as partidas em andamento), matchmaking (partidas nascem pela `Registry`, não por endpoint) e **condição de vitória** — `EngineError::GameOver` é declarado e nunca produzido, então "jogar até o fim" ainda não significa nada. Fechar essa última é decisão de regra, não de transporte.

---

## 6. Cartas como dados

Cada carta é uma entrada declarativa, versionada em git, com um identificador estável. O conjunto tem versão semântica e é **imutável depois de publicado** — nunca edite uma carta publicada, crie uma nova versão e um novo conjunto. Motivo: replay de uma partida de um ano atrás precisa rodar no conjunto da época.

A DSL cobre os casos comuns: custo, tipo, palavras-chave, efeito composto de primitivas (dançar dano, comprar, destruir, criar ficha, modificar característica). Um validador em `vertice-tools` roda no CI e rejeita carta malformada, custo fora de curva ou referência a palavra-chave inexistente.

**O id é do registro, nunca da posição (M1.2).** `cards/registry.toml` guarda `slug -> CardId` e é **append-only**: id novo só anexa no fim, slug não sai, id não se reatribui. O `cards/registry.lock` guarda o hash do conteúdo de cada carta publicada, que é o que transforma "carta publicada não se edita" de convenção em portão. O `next_id` no registro é a marca que sobrevive fora do lock: sem ela, apagar uma carta do registro, do arquivo **e** do lock deixaria tudo verde e o id apagado voltaria a parecer livre. Com ela — o validador exige a faixa `0..next_id` inteira preenchida — deleção vira erro. `vertice-tools validate` recusa inserção no meio, deleção, remoção de slug, reuso de id e edição de carta publicada; `generate` compila o TOML em `vertice-core/src/generated/cards.rs`, que é **commitado** — o motor continua com zero dependências e o build não depende de rodar ferramenta. Antes disto, `CardId` era o índice do array: mexer na ordem do catálogo trocava uma carta por outra em silêncio e quebrava todo replay existente.

**A curva de custo tem número, e ele mora num lugar só.** Referência: criatura usa o **menor** entre força e resistência como base, mais 1 se tiver palavra-chave; mágica de dano N custa ⌈N/2⌉ (2 de dano por 1 mana é o teto); compra de N custa N; pump segue a régua do dano. Banda de ±1 mana. Passar da banda não bloqueia a carta: exige `justificativa` no próprio arquivo, que é o escape que a referência de conteúdo já previa. O menor valor como base é a leitura que fecha com as 13 cartas já publicadas sem nenhuma justificativa. Código em `vertice-tools/src/curve.rs`.

O escape pra cartas esquisitas é **script WASM por carta**, carregado no servidor. Custa mais e é mais lento, então é exceção — se 20% das cartas precisam de WASM, sua DSL está fraca.

**Tamanho de pool.** Isto é o orçamento real de conteúdo, e ele é maior do que você imagina:

- 200–250 cartas: mínimo pra decks-padrão por eco funcionarem (que é como o jogo começa, e é o que a lore já te dá de graça — o eco entrega a lista canônica).
- 600–800 cartas: piso pra deckbuilding livre em Commander singleton ser divertido. Com 100 cartas por deck e pool pequeno, todos os decks ficam iguais.
- ~150 cartas por set sazonal.

Ou seja: lance com **decks-padrão por eco e pool pequeno**, e cresça o pool por temporada. Isso é uma decisão de design, não uma limitação — e é a única forma de um solo ter conteúdo suficiente.

---

## 7. Bot

Você precisa de IA boa o bastante pra sustentar uma campanha inteira de escola. Não precisa de IA forte.

**Fase 1: heurística.** Avaliação de tabuleiro por pesos, curva de mana, alvo por ameaça, e regras específicas por arquétipo de eco. Um bot heurístico decente bate jogador médio no começo e é o que dá pra fazer sozinho.

**Fase 2: busca.** Monte Carlo com poucas simulações, rodando no core determinístico, quando o motor já estiver estável. Isso só funciona se o core for rápido e puro — outra razão pra manter o core limpo.

**Não use LLM pra jogar.** Custa, atrasa, e é inconsistente. LLM serve pra escrever sabor e diálogo, não pra pilotar deck.

Dificuldade por faixa: o NPC calouro joga mal de propósito em decisões específicas (não usa a melhor jogada sempre), e o formando joga sério. Transparência: o jogador tem que sentir a diferença, não ser informado dela.

---

## 8. A alternativa que você deve rejeitar de olhos abertos

**Godot 4 é objetivamente um motor de jogo melhor que React + Pixi.** Ele te dá tilemap, animação de sprite, input, áudio, cena, e export nativo pra desktop — sem Tauri, sem glue, sem launcher. Se você fosse um dev de jogo, eu te mandaria pra lá sem pensar.

Eu não te mando por um motivo só: **você é dev web, e dev solo não tem seis meses pra aprender motor de jogo enquanto constrói motor de regras.** Somado a isso, o deck builder e a busca de carta são telas densas de interface, e React é brutalmente melhor nisso que qualquer UI de Godot.

Registre a decisão e o motivo. Se em seis meses você descobrir que a camada Pixi está te custando mais do que economizou, você reavalia com informação. Não reavalie agora, no papel.

---

## 9. Arte — o risco que mata solo

O motor é difícil, mas arte é o que **para** projetos solo. Um jogo de carta com 300 ilustrações é impossível sozinho, e chibi de overworld ainda pede sprite sheet, animação e retrato por personagem.

Duas saídas que funcionam:

**Direção iconográfica.** Em vez de ilustração pintada por carta, um sistema visual de sigilo — geometria, cor, ícone, tipografia. Custa uma fração, é consistente, e te dá identidade. Muitos jogos digitais de carta fizeram isso por necessidade e ficaram melhores que o genérico pintado. Você pode investir em **poucas** ilustrações de alta qualidade só nos comandantes, que são o que o jogador olha.

**Pool pequeno por temporada.** Já decidido acima. Menos carta, menos arte.

O resto (chibi, cenário, interface) é terceirizável por peça, e deve ser — motor é seu, arte não precisa ser. Reserve orçamento pra duas coisas: os comandantes e a transição de cena do overworld pro duelo, que é o momento de maior valor de produção do jogo.

---

## 10. Marcos

Cada marco tem critério de saída. Sem o critério cumprido, não avança.

**M0 — O motor respira** (o mais importante)
Core Rust com estado, ações, resolução, redação. Duas listas de 100 cartas fixas. Duelo 1v1 hot-seat num browser, sem arte, sem rede, sem banco.
*Sai quando:* uma partida completa roda do começo ao fim, o replay reproduz bit a bit, e o CI barra não-determinismo.
*Aqui você descobre se o seu conjunto de regras é implementável. Se for insuportável, é agora que você corta mais — não no M3.*

**M1 — Carta como dado**
DSL, validador, pipeline de compilação, 250 cartas, deck builder funcional com busca. Servidor com partida autoritativa 1v1 por WebSocket, reconexão, Postgres.
*Sai quando:* dois navegadores jogam uma partida real, um cai, volta, e continua.

**M2 — O jogo tem cara de jogo**
Pixi: mapa com chibi, transição de cena, mesa de duelo. Bot heurístico. Duelo de NPC offline via WASM.
*Sai quando:* dá pra jogar uma hora sem sentir que é protótipo.

**M3 — Quatro jogadores**
Modo pod, interface de quatro, redação por espectador, espectador e replay público.
*Sai quando:* quatro pessoas jogam uma partida inteira sem quebrar.

**M4 — O Vértice**
Calendário, aula, ficha, fama, vínculo, ecos, temperamento, rancor. Ato I completo.
*Sai quando:* a campanha tem começo, meio e fim, e o jogador escolhe entre duas rotas.

**M5 — Produto**
Tauri, Steam, cosmético, loja, telemetria mínima.
*Sai quando:* um estranho compra, instala e joga sem você falar com ele.

---

## 11. O que não fazer agora

Não construa o launcher. Não construa loja. Não construa fila ranqueada. Não construa espectador. Não construa editor de carta pro jogador. Não construa sistema de torneio. Não faça mobile. Não faça localização. Não faça 3D.

Cada um desses é uma semana a um mês e nenhum deles é o que faz o jogo existir.

---

## 12. Riscos, sem eufemismo

**O motor é maior do que parece.** Um conjunto de regras de Commander é um projeto de anos. Os cortes da seção 4 são o que traz isso pra um território possível — e mesmo com todos eles, é o maior item do seu orçamento.

**Arte é o teto real.** Já tratado. A decisão iconográfica não é estética, é de sobrevivência.

**Solo é solo.** Tempo integral, vertical slice do M0 ao M2 é 9 a 12 meses de trabalho competente e contínuo. O escopo completo — quatro jogadores, campanha de quatro atos, online, monetização, Steam — é 3 a 4 anos. Meio período, dobre. Eu não recomendaria mentir pra si mesmo nesse número, porque é ele que define se você termina.

**O modo de quatro jogadores é onde o sonho costuma morrer.** Ele é o coração do formato e é o dobro de complexidade de tudo. Se em algum momento você precisar cortar algo grande, corte o pod antes de cortar a campanha — o jogo é sobre a escola, não sobre a mesa de quatro. Dói, mas é a escolha que salva o projeto.

**Não abra a campanha antes do motor estar estável.** Escrever os quatro atos sobre regras que ainda vão mudar é o jeito mais eficiente de jogar texto fora.

---

## 13. Nomenclatura

**Nome do jogo: Vértice.** A escola, o nó das leylines, o ponto onde tudo se cruza. Nome próprio, curto, e é literalmente o que o jogo é.

**Identificador técnico: `vertice`, sem acento.** Nome de crate, de pacote, de binário e de pasta nunca leva acento — acento em identificador gera problema de encoding em CI, em Docker, em path de Windows e em nome de pacote npm. A regra é fixa:

| Contexto | Grafia |
|---|---|
| Texto para humano (título, UI, doc) | `Vértice` |
| Identificador técnico (crate, pasta, binário, slug) | `vertice` |
| Crates do projeto | `vertice-core`, `vertice-server`, `vertice-client`, `vertice-bot`, `vertice-tools`, `vertice-wasm` |

### A decisão registrada: por que não Hazoret

Hazoret foi considerado e rejeitado. Hazoret the Fervent é personagem da Wizards of the Coast, deusa vermelha de Amonkhet, publicada desde 2017.

Rejeitado por dois motivos, nessa ordem de peso:

1. **Contradiz a premissa de IP própria.** A arquitetura inteira existe para que eco seja entrada de dados e a IP possa ser trocada em três campos. Um título emprestado joga isso fora e sinaliza ao público que o jogo é de Magic — o que atrai exatamente a fiscalização que a IP própria evitava.
2. **Risco de marca.** Título de produto é a categoria mais protegida. Steam, itch e lojas de aplicativo derrubam por notificação do detentor, e não é preciso ganhar ou perder ação judicial para o jogo sair do ar na semana de lançamento.

**Regra que fica de pé:** nenhum nome de personagem de franquia existente entra neste projeto — nem como título, nem como personagem, nem como carta, nem como referência interna em documento público. Se o eco algum dia precisar ser reskinado para uma IP licenciada, o nome atual não pode ser um obstáculo.

**Pendência honesta:** "Vértice" é palavra comum do português e existe empresa e produto chamado Vertex/Vértice em outros mercados. Eu **não** verifiquei disponibilidade de marca. Antes de registrar domínio, abrir Steam page ou depositar no INPI, faça busca de anterioridade na classe de software/jogos. Isso é verificação de escritório de marcas, não de arquiteto de software — e é barata agora, caríssima depois.
