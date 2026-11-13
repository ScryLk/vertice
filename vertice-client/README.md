# vertice-client — M1.4

Cliente mínimo: React + Vite, sem Pixi, sem arte. Existe para provar o
protocolo na interface antes de investir em visual.

`src/protocol.ts` é o espelho escrito à mão dos DTOs de
`vertice-server/src/protocol/`. Quando o servidor mudar a forma,
`PROTOCOL_VERSION` sobe e este arquivo tem que acompanhar.

## Rodar

Dois processos. Primeiro o servidor, da raiz do repositório:

```bash
VERTICE_DATA=data/partidas PORT=8080 cargo run -p vertice-server
```

Depois o cliente:

```bash
npm --prefix vertice-client install
npm --prefix vertice-client run dev
```

Abra http://localhost:5173. Dá para criar uma partida de dois jogadores (abra
os dois assentos em abas diferentes) ou jogar direto contra o bot, no nível
`calouro` ou `formando`.

Testes: `npm --prefix vertice-client test` (Vitest + Testing Library, visões
fixas, sem servidor).

O Vite faz proxy de `/matches` e `/ws` para `127.0.0.1:8080`, então **não
existe CORS no servidor**: em desenvolvimento o navegador fala com o Vite, e
em produção os dois saem da mesma origem.

## O que esta tela não tem, de propósito

Pixi, animação, transição, chibi, deck building, som. Tudo isso é M2 em
diante. O que está na tela é o protocolo — se algo doer, tem que doer aqui,
enquanto ainda é barato mudar.
