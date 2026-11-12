//! Servidor autoritativo do Vértice.
//!
//! O motor (`vertice-core`) é a verdade. Este crate não decide regra nenhuma:
//! transporta `Action` para dentro, `PlayerView` para fora, e mantém a
//! partida viva enquanto alguém estiver conectado.
//!
//! Três camadas, e cada uma só conhece a de baixo:
//!
//! - [`protocol`] — tradução entre os tipos do motor e a forma de fio.
//! - [`actor`] — uma task por partida, dona do `GameState`. É a autoridade.
//! - [`http`] — WebSocket. Burra de propósito: lê quadros e repassa.
//!
//! Duas invariantes que valem aqui como valem no core:
//!
//! - **A seta aponta para dentro.** Este crate conhece `vertice-core`;
//!   `vertice-core` não sabe que este crate existe.
//! - **O cliente nunca recebe `GameState`.** Só `PlayerView`, e só a do
//!   próprio assento. A redação acontece no motor; o servidor não a refaz e
//!   não a desfaz.

pub mod actor;
pub mod http;
pub mod protocol;
pub mod registry;
pub mod store;

pub use http::router;
pub use protocol::PROTOCOL_VERSION;
pub use registry::Registry;
pub use store::Store;
