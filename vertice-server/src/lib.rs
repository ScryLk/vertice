//! Servidor autoritativo do Vértice.
//!
//! O motor (`vertice-core`) é a verdade. Este crate não decide regra nenhuma:
//! ele transporta `Action` para dentro, `PlayerView` para fora, e guarda o
//! `MatchRecord` para a partida sobreviver a uma queda de conexão.
//!
//! Por enquanto existe só o **protocolo** — a tradução entre os tipos do
//! motor e a forma que atravessa a rede. A camada de transporte (Axum,
//! WebSocket, uma task por partida) entra em cima dele, sem tocá-lo.
//!
//! Duas invariantes que valem aqui como valem no core:
//!
//! - **A seta aponta para dentro.** Este crate conhece `vertice-core`;
//!   `vertice-core` não sabe que este crate existe.
//! - **O cliente nunca recebe `GameState`.** Só `PlayerView`, e só a do
//!   próprio assento. A redação acontece no motor; o servidor não a refaz e
//!   não a desfaz.

pub mod protocol;

pub use protocol::PROTOCOL_VERSION;
