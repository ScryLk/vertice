//! Peças compartilhadas entre os testes de integração do servidor.
//!
//! `#![allow(dead_code)]` porque cada arquivo de `tests/` compila este módulo
//! por conta própria: o que um teste usa, o outro pode não usar, e sem isso o
//! portão do clippy reprova por item não utilizado. Silenciar dead code aqui é
//! seguro — é módulo de teste, não código de produção.
#![allow(dead_code)]

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

pub type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Quanto esperar por um quadro antes de considerar o servidor travado.
const ESPERA: Duration = Duration::from_secs(5);

pub fn baralho(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

/// Sobe o servidor numa porta efêmera e devolve o endereço.
pub async fn servir(registry: Arc<Registry>) -> SocketAddr {
    let app = router(registry);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("porta efêmera livre");
    let addr = listener.local_addr().expect("endereço local");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("servidor de teste");
    });

    addr
}

/// Cliente que fala o protocolo como um cliente de verdade falaria: só JSON
/// na entrada e na saída, sem atalho pelo lado de dentro.
pub struct Client {
    pub socket: Socket,
    pub seq: u64,
    pub view: PlayerViewDto,
}

impl Client {
    pub async fn connect(addr: SocketAddr, match_id: &str, seat: PlayerId) -> Client {
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

    pub async fn send(&mut self, message: &ClientMessage) {
        send(&mut self.socket, message).await;
    }

    pub async fn act(&mut self, action: ActionDto) {
        let message = ClientMessage::Action {
            seq: self.seq,
            action,
        };
        self.send(&message).await;
    }

    pub async fn next(&mut self) -> ServerMessage {
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

pub async fn send(socket: &mut Socket, message: &ClientMessage) {
    let text = serde_json::to_string(message).expect("serializa");
    socket
        .send(Message::Text(text.into()))
        .await
        .expect("envia");
}

pub async fn receive(socket: &mut Socket) -> ServerMessage {
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
pub async fn passo(actor: &mut Client, other: &mut Client) -> bool {
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

pub async fn jogar(a: &mut Client, b: &mut Client, limite: usize) -> usize {
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
