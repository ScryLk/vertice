//! O Vértice — motor de regras.
//!
//! Regras deste crate, válidas para sempre:
//!
//! 1. Zero I/O. Nada de rede, disco, terminal.
//! 2. Zero não-determinismo ambiente. Sem `thread_rng`, sem `SystemTime`,
//!    sem `HashMap` (ordem de iteração não é estável). RNG e tempo entram
//!    por parâmetro.
//! 3. O estado é a verdade. Ações entram, estado muda, view redigida sai.
//! 4. Dado (decks, seed, ações), a partida reproduz bit a bit.
//!
//! `scripts/check-determinism.sh` falha o CI se a regra 2 for violada.

#![forbid(unsafe_code)]

mod generated;

pub mod action;
pub mod cards;
pub mod engine;
pub mod ids;
pub mod record;
pub mod rng;
pub mod state;
pub mod view;

pub use action::Action;
pub use engine::{apply, legal_actions, new_game, new_game_with_hands, EngineError};
pub use ids::{CardId, ObjectId, PlayerId};
pub use record::{replay, MatchRecord};
pub use state::{GameState, Object, Phase, Player, Target, Zones};
pub use view::{view, PlayerView};

/// Versão do motor. Sobe quando o *comportamento* muda, mesmo sem mudar API.
///
/// 0.2.0 acrescentou fim de partida: vida <= 0 encerra o jogo e `winner`
/// passa a fazer parte do estado. Um `MatchRecord` da 0.1.0 não significa a
/// mesma coisa, então a versão tinha que dizer isso.
pub const ENGINE_VERSION: &str = "0.2.0";
pub const CARD_SET_VERSION: &str = "m0.1";

/// Vida inicial. Comandante usa 40.
pub const STARTING_LIFE: i32 = 40;
/// Tamanho obrigatório de deck no M0.
pub const DECK_SIZE: usize = 100;
/// Cartas na mão inicial.
pub const OPENING_HAND: usize = 7;
