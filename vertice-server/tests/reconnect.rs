//! O marco da M1.3, automatizado.
//!
//! Dois clientes WebSocket headless jogam a mesma partida. Um deles tem a
//! conexão derrubada no meio. Ele reconecta e a partida continua exatamente
//! de onde estava — mesmo `seq`, mesma `view`, mesmo `fingerprint`.
//!
//! "Abri dois navegadores e funcionou" não é prova, e este arquivo existe
//! para nunca precisar dessa frase.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use vertice_core::cards::CATALOG_LEN;
use vertice_core::{CardId, PlayerId};
use vertice_server::protocol::*;
use vertice_server::{router, Registry, PROTOCOL_VERSION};

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

fn baralho(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

/// Cliente que fala o protocolo como um cliente de verdade falaria: só JSON
/// na entrada e na saída, sem atalho pelo lado de dentro.
struct Client {
    socket: Socket,
    seq: u64,
    view: PlayerViewDto,
}

impl Client {
    async fn connect(addr: SocketAddr, match_id: &str, seat: PlayerId) -> Client {
        let (mut socket, _) = connect_async(format!("ws://{addr}/ws/{match_id}"))
            .await
            .expect("abre o WebSocket");

        let join = ClientMessage::Join {
            protocol: PROTOCOL_VERSION,
            seat,
        };
        send(&mut socket, &join).await;

        match receive(&mut socket).await {
            ServerMessage::Welcome { seq, view, .. } => Client { socket, seq, view },
            other => panic!("esperava `Welcome`, veio {other:?}"),
        }
    }

    async fn send(&mut self, message: &ClientMessage) {
        send(&mut self.socket, message).await;
    }

    async fn act(&mut self, action: ActionDto) {
        let message = ClientMessage::Action {
            seq: self.seq,
            action,
        };
        self.send(&message).await;
    }

    async fn next(&mut self) -> ServerMessage {
        let message = receive(&mut self.socket).await;
        match &message {
            ServerMessage::View { seq, view } | ServerMessage::Welcome { seq, view, .. } => {
                self.seq = *seq;
                self.view = view.clone();
            }
            _ => {}
        }
        message
    }
}

async fn send(socket: &mut Socket, message: &ClientMessage) {
    let text = serde_json::to_string(message).expect("serializa");
    socket
        .send(Message::Text(text.into()))
        .await
        .expect("envia");
}

/// Quanto esperar por um quadro antes de considerar o servidor travado.
const ESPERA: Duration = Duration::from_secs(5);

async fn receive(socket: &mut Socket) -> ServerMessage {
    loop {
        // Sem timeout, uma regressão no servidor faz o teste *pendurar* — e
        // teste que pendura esconde o problema atrás de um CI travado, que é
        // pior que um teste vermelho.
        let frame = tokio::time::timeout(ESPERA, socket.next())
            .await
            .unwrap_or_else(|_| {
                panic!("o servidor não mandou nada em {ESPERA:?}: travou ou parou de notificar")
            });

        match frame {
            Some(Ok(Message::Text(text))) => {
                return serde_json::from_str(&text).expect("o servidor só manda JSON válido")
            }
            Some(Ok(_)) => continue,
            other => panic!("conexão caiu: {other:?}"),
        }
    }
}

/// Um passo: quem tem a prioridade joga a primeira ação legal e os dois
/// assentos recebem a view nova.
async fn passo(actor: &mut Client, other: &mut Client) -> bool {
    let Some(action) = actor.view.legal_actions.first().copied() else {
        return false;
    };
    actor.act(action).await;

    let mine = actor.next().await;
    let theirs = other.next().await;
    assert!(
        matches!(mine, ServerMessage::View { .. }),
        "esperava `View` para quem agiu, veio {mine:?}"
    );
    assert!(
        matches!(theirs, ServerMessage::View { .. }),
        "o outro assento também precisa ser avisado, veio {theirs:?}"
    );
    true
}

async fn jogar(a: &mut Client, b: &mut Client, limite: usize) -> usize {
    let mut feitos = 0;
    for _ in 0..limite {
        let acted = if a.view.priority == 0 {
            passo(&mut *a, &mut *b).await
        } else {
            passo(&mut *b, &mut *a).await
        };
        if !acted {
            break;
        }
        feitos += 1;
    }
    feitos
}

async fn subir_servidor() -> (SocketAddr, Arc<Registry>) {
    let registry = Arc::new(Registry::new());
    let app = router(registry.clone());
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("porta efêmera livre");
    let addr = listener.local_addr().expect("endereço local");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("servidor de teste");
    });

    (addr, registry)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partida_sobrevive_a_queda_de_um_dos_clientes() {
    let (addr, registry) = subir_servidor().await;
    let id = registry
        .create(baralho(0), baralho(3), 2024)
        .await
        .expect("cria a partida");

    let mut a = Client::connect(addr, &id, 0).await;
    let mut b = Client::connect(addr, &id, 1).await;

    let andados = jogar(&mut a, &mut b, 15).await;
    assert!(
        andados >= 10,
        "a partida precisa andar de verdade antes da queda; andou {andados} passos"
    );
    assert!(
        a.view.winner.is_none(),
        "a queda tem que ser testada com a partida viva, não encerrada"
    );

    let fingerprint_antes = registry.fingerprint(&id).await.expect("fingerprint");
    let view_antes = a.view.clone();
    let seq_antes = a.seq;
    assert_eq!(
        a.seq, b.seq,
        "os dois assentos têm que concordar sobre o índice"
    );

    // A queda.
    drop(a);

    // A volta: o mesmo assento, uma conexão nova.
    let mut a = Client::connect(addr, &id, 0).await;

    assert_eq!(a.seq, seq_antes, "reconexão tem que voltar no mesmo índice");
    assert_eq!(
        a.view, view_antes,
        "a view depois de reconectar tem que ser exatamente a de antes"
    );
    assert_eq!(
        registry.fingerprint(&id).await.expect("fingerprint"),
        fingerprint_antes,
        "o estado do motor não pode ter se mexido enquanto ninguém estava olhando"
    );

    // E a partida continua de onde parou.
    let depois = jogar(&mut a, &mut b, 15).await;
    assert!(
        depois > 0,
        "a partida tem que continuar depois da reconexão; andou {depois} passos"
    );
    assert!(a.seq > seq_antes, "o índice tem que avançar");
    assert_eq!(a.seq, b.seq, "os dois assentos continuam de acordo");
    assert_eq!(
        a.view.winner, b.view.winner,
        "se a partida acabou nesse meio-tempo, os dois têm que saber igual"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partida_joga_ate_o_fim_e_os_dois_assentos_sabem_quem_venceu() {
    let (addr, registry) = subir_servidor().await;
    let id = registry
        .create(baralho(0), baralho(3), 2024)
        .await
        .expect("cria a partida");

    let mut a = Client::connect(addr, &id, 0).await;
    let mut b = Client::connect(addr, &id, 1).await;

    let mut passos = 0;
    while a.view.winner.is_none() && passos < 400 {
        let acted = if a.view.priority == 0 {
            passo(&mut a, &mut b).await
        } else {
            passo(&mut b, &mut a).await
        };
        if !acted {
            break;
        }
        passos += 1;
    }

    let vencedor = a.view.winner.expect("a partida tem que terminar");
    assert_eq!(
        b.view.winner,
        Some(vencedor),
        "os dois assentos veem o mesmo resultado"
    );
    assert!(
        a.view.legal_actions.is_empty(),
        "partida encerrada não oferece ação"
    );
    assert!(b.view.legal_actions.is_empty());

    // E o servidor recusa com a razão certa, não com "ação ilegal".
    let seq = a.seq;
    a.send(&ClientMessage::Action {
        seq,
        action: ActionDto::EndTurn,
    })
    .await;
    match a.next().await {
        ServerMessage::Rejected { reason, .. } => assert_eq!(reason, RejectReason::GameOver),
        other => panic!("esperava `Rejected(GameOver)`, veio {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn o_servidor_recusa_acao_ilegal_sem_mexer_no_estado() {
    let (addr, registry) = subir_servidor().await;
    let id = registry
        .create(baralho(0), baralho(3), 77)
        .await
        .expect("cria a partida");

    let mut a = Client::connect(addr, &id, 0).await;
    let antes = registry.fingerprint(&id).await.expect("fingerprint");

    // Objeto que não existe: fora de `legal_actions`.
    let seq = a.seq;
    a.send(&ClientMessage::Action {
        seq,
        action: ActionDto::PlayLand { object: 9999 },
    })
    .await;
    match a.next().await {
        ServerMessage::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::IllegalAction);
        }
        other => panic!("esperava `Rejected`, veio {other:?}"),
    }
    assert_eq!(
        registry.fingerprint(&id).await.expect("fingerprint"),
        antes,
        "ação recusada não pode mexer no estado"
    );

    // Índice que o servidor não reconhece: o cliente está prevendo em cima de
    // um passado que não existe mais.
    let seq = a.seq + 50;
    a.send(&ClientMessage::Action {
        seq,
        action: ActionDto::EndTurn,
    })
    .await;
    match a.next().await {
        ServerMessage::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::Stale);
        }
        other => panic!("esperava `Rejected`, veio {other:?}"),
    }
    assert_eq!(
        registry.fingerprint(&id).await.expect("fingerprint"),
        antes,
        "índice atrasado também não pode mexer no estado"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partida_inexistente_e_recusada() {
    let (addr, _registry) = subir_servidor().await;

    let (mut socket, _) = connect_async(format!("ws://{addr}/ws/nao-existe"))
        .await
        .expect("abre o WebSocket");
    send(
        &mut socket,
        &ClientMessage::Join {
            protocol: PROTOCOL_VERSION,
            seat: 0,
        },
    )
    .await;

    match receive(&mut socket).await {
        ServerMessage::Error { kind } => assert_eq!(kind, ErrorKind::UnknownMatch),
        other => panic!("esperava `Error`, veio {other:?}"),
    }
}
