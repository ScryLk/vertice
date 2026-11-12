//! O transporte. WebSocket, um por conexão, e nada mais.
//!
//! A conexão é burra de propósito: ela lê quadros, traduz para `Command` e
//! manda para a task da partida; e escreve o que a task empurrar. Quem decide
//! é o ator. Isso é o que faz cair a conexão não significar nada.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{Sink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use vertice_core::PlayerId;

use crate::actor::Command;
use crate::protocol::{ClientMessage, ErrorKind, ServerMessage, PROTOCOL_VERSION};
use crate::registry::Registry;

pub type SharedRegistry = Arc<Registry>;

pub fn router(registry: SharedRegistry) -> Router {
    Router::new()
        .route("/matches", post(create_match))
        .route("/ws/{match_id}", get(upgrade))
        .with_state(registry)
}

/// Pedido de criação de partida.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CreateMatchRequest {
    /// Semente do embaralhamento. Ausente = derivada do número da partida.
    ///
    /// Vem do cliente de propósito: é a única coisa que ele pode escolher, e
    /// fica registrada no `MatchRecord`, então o replay continua exato.
    #[serde(default)]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMatchResponse {
    pub match_id: String,
    pub protocol: u32,
}

/// `POST /matches` — cria a mesa.
///
/// Exige corpo JSON (`{}` serve). Matchmaking de verdade é outro problema;
/// isto existe para um cliente conseguir entrar num jogo.
async fn create_match(
    State(registry): State<SharedRegistry>,
    Json(request): Json<CreateMatchRequest>,
) -> Result<Json<CreateMatchResponse>, (StatusCode, String)> {
    let match_id = registry
        .create_default(request.seed)
        .await
        .map_err(|error| (StatusCode::BAD_REQUEST, format!("{error:?}")))?;

    Ok(Json(CreateMatchResponse {
        match_id,
        protocol: PROTOCOL_VERSION,
    }))
}

async fn upgrade(
    upgrade: WebSocketUpgrade,
    Path(match_id): Path<String>,
    State(registry): State<SharedRegistry>,
) -> Response {
    upgrade.on_upgrade(move |socket| session(socket, match_id, registry))
}

async fn session(mut socket: WebSocket, match_id: String, registry: SharedRegistry) {
    let Some(actor) = registry.handle(&match_id).await else {
        let _ = send_message(&mut socket, &ServerMessage::Error { kind: ErrorKind::UnknownMatch }).await;
        return;
    };

    // O primeiro quadro é o `Join`. Antes dele não existe assento, e recusar
    // cedo evita manter aberta uma conexão que não sabe falar o protocolo.
    let Some(ClientMessage::Join { protocol, seat }) = first_message(&mut socket).await else {
        let _ = send_message(
            &mut socket,
            &ServerMessage::Error { kind: ErrorKind::ProtocolMismatch },
        )
        .await;
        return;
    };
    if protocol != PROTOCOL_VERSION || seat > 1 {
        let _ = send_message(
            &mut socket,
            &ServerMessage::Error { kind: ErrorKind::ProtocolMismatch },
        )
        .await;
        return;
    }

    let (outbound, mut pushed) = mpsc::channel::<ServerMessage>(64);
    let (reply, answer) = oneshot::channel();
    if actor
        .send(Command::Attach { seat, outbound, reply })
        .await
        .is_err()
    {
        return;
    }
    let Ok(attached) = answer.await else { return };
    let generation = attached.generation;

    let welcome = ServerMessage::Welcome {
        protocol: PROTOCOL_VERSION,
        seat,
        seq: attached.seq,
        view: attached.view,
    };
    if send_message(&mut socket, &welcome).await.is_err() {
        return;
    }

    let (mut sink, mut stream) = socket.split();

    // Escritor numa task própria: assim a leitura nunca fica presa esperando
    // o cliente consumir, e o `select!` de empréstimos duplos deixa de existir.
    let writer = tokio::spawn(async move {
        while let Some(message) = pushed.recv().await {
            if send_message(&mut sink, &message).await.is_err() {
                break;
            }
        }
    });

    while let Some(frame) = stream.next().await {
        let Ok(Message::Text(text)) = frame else { break };

        match serde_json::from_str::<ClientMessage>(text.as_str()) {
            Ok(ClientMessage::Action { seq, action }) => {
                if actor.send(Command::Act { seat, seq, action }).await.is_err() {
                    break;
                }
            }
            Ok(ClientMessage::Resync { .. }) => {
                if actor.send(Command::Snapshot { seat }).await.is_err() {
                    break;
                }
            }
            // `Join` de novo no meio da conversa, ou lixo: a conexão acabou.
            Ok(ClientMessage::Join { .. }) | Err(_) => break,
        }
    }

    writer.abort();
    // O `Detach` leva a geração: se este cliente já reconectou, a conexão
    // nova tem geração maior e este comando não apaga o canal dela.
    let _ = actor.send(Command::Detach { seat, generation }).await;
}

async fn first_message(socket: &mut WebSocket) -> Option<ClientMessage> {
    // Alguns quadros de controle chegam antes do primeiro texto; ignorar é
    // mais gentil que derrubar a conexão por causa de um `Ping`.
    for _ in 0..8 {
        match socket.next().await? {
            Ok(Message::Text(text)) => return serde_json::from_str(text.as_str()).ok(),
            Ok(_) => continue,
            Err(_) => return None,
        }
    }
    None
}

async fn send_message<S>(sink: &mut S, message: &ServerMessage) -> Result<(), ()>
where
    S: Sink<Message> + Unpin,
{
    let text = serde_json::to_string(message).map_err(|_| ())?;
    sink.send(Message::Text(text.into())).await.map_err(|_| ())
}

/// Assento válido é 0 ou 1. Existe como função só para o teste de protocolo
/// poder usar a mesma regra que o servidor.
pub fn seat_is_valid(seat: PlayerId) -> bool {
    seat < 2
}
