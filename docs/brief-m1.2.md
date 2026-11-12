# Prompt — Claude Code, fatia M1.2

Cole o bloco abaixo na sessão do Claude Code, na raiz do repositório.

---

## Tarefa: implementar a fatia M1.2 — carta como dado

Você está no repositório do **Vértice**, um jogo de duelo de comandantes com
motor de regras em Rust. Leia, nesta ordem, e só isso antes de começar:

1. `CLAUDE.md` (raiz) — regras invioláveis, mapa de arquivos, protocolo de correção
2. `docs/roadmap.md`, seção **M1.2 Carta como dado**
3. `vertice-core/src/cards.rs` e `vertice-core/src/ids.rs`

Não varra o resto do repositório antes de terminar esses três.

### Baseline antes de tocar em qualquer coisa

```bash
cd vertice-core && cargo test && ./scripts/check-determinism.sh
cargo clippy --all-targets -- -D warnings
```

Esperado: 14 testes verdes. Se não estiver verde, **pare e me avise** — não
conserte o que já estava quebrado.

Crie a branch antes do primeiro commit: `git checkout -b m1.2-carta-como-dado`.

### Objetivo

Tirar o catálogo de cartas do código e colocá-lo em dado versionado, com
identidade estável que nunca muda. Hoje `CardId` é a posição no array: inserir
uma carta no meio desloca todos os ids seguintes e quebra todo replay
existente, em silêncio.

### O que fazer

1. **Workspace.** Criar `Cargo.toml` na raiz com
   `[workspace] members = ["vertice-core", "vertice-tools"]`, resolver v2.
   Sem isso `cargo run -p` não funciona.

2. **Registro.** Criar `cards/registry.toml`: tabela `slug -> id`, append-only.
   O formato é a garantia: id nunca reutilizado, slug nunca removido. O
   validador recusa as duas coisas.

3. **Cartas.** Criar `cards/set-m1/*.toml`, uma carta por arquivo, com slug,
   nome, custo, tipo, força, resistência, palavras-chave e efeito. Migrar as
   13 cartas que estão hoje em `vertice-core/src/cards.rs` sem mudar nenhum
   comportamento.

4. **Validador e gerador.** Criar `vertice-tools/` com dois comandos:
   - `validate` — custo na curva (`arquitetura.md` §4 e `roadmap.md` M1.2),
     palavra-chave conhecida, id único, slug presente no registro, nenhuma
     remoção, nenhuma carta publicada alterada (hash por carta em
     `cards/registry.lock`)
   - `generate` — emite `vertice-core/src/generated/cards.rs`

5. **Ligar.** `vertice-core/src/cards.rs` passa a reexportar o gerado,
   mantendo a API pública idêntica: `CATALOG`, `CATALOG_LEN`, `card(id)`,
   `CardDef`, `Keyword`, `CardKind`, `Effect`. O arquivo gerado é **commitado**
   — o build não pode depender de rodar a ferramenta.

6. **Teste de regressão do registro.** Provar que inserir carta no meio do
   conjunto é **recusado pelo validador**, não apenas desencorajado por
   documentação.

### Restrições

- `vertice-core` continua com **zero dependências**. O TOML é lido por
  `vertice-tools` em tempo de validação/build, nunca em tempo de jogo. Se
  precisar de dependência no core, a solução está errada — pare.
- **Não altere regra de jogo.** `engine.rs` não é seu nesta fatia.
- **Não reordene `CardId`.** Id vem do registro, nunca do índice.
- **Não avance para M1.3.**

### Como trabalhar

- Teste que falha primeiro, correção depois.
- Se aparecer decisão que não está escrita aqui nem no roadmap, **pare e
  pergunte** em vez de decidir sozinho.
- Um commit no fim, assunto: `M1.2: carta como dado`.

### Fechamento

1. Rode os três portões e cole a saída.
2. Rode `cargo run -p vertice-tools -- validate` e cole a saída.
3. Prove estabilidade de id: gere o código duas vezes e compare byte a byte.
4. Peça ao subagente `determinism-guard` uma auditoria do diff e anexe o
   veredito.
5. Relatório no formato do `CLAUDE.md` §8, trocando `SINTOMA` por `OBJETIVO`.
6. Liste o que ficou de fora de propósito.

Não peça confirmação a cada passo: trabalhe até o fim da fatia e entregue o
relatório. Pare apenas nas condições de parada acima.
