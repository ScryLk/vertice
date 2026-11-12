//! Registro de partida e replay.
//!
//! Uma partida inteira é reproduzível a partir de:
//!   (engine_version, card_set_version, seed, deck0, deck1, actions)
//!
//! Nada mais. Nem estados intermediários, nem snapshots, nem diffs. Isso
//! mantém o banco pequeno, torna espectador e replay triviais, e — o mais
//! importante — dá um teste de regressão real: se o motor mudar e a mesma
//! partida não reproduzir, você quebrou compatibilidade e precisa saber.

use crate::action::Action;
use crate::engine::{apply, new_game, EngineError};
use crate::ids::CardId;
use crate::state::GameState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchRecord {
    pub engine_version: &'static str,
    pub card_set_version: &'static str,
    pub seed: u64,
    pub deck0: Vec<CardId>,
    pub deck1: Vec<CardId>,
    pub actions: Vec<Action>,
}

impl MatchRecord {
    pub fn new(deck0: Vec<CardId>, deck1: Vec<CardId>, seed: u64) -> Self {
        Self {
            engine_version: crate::ENGINE_VERSION,
            card_set_version: crate::CARD_SET_VERSION,
            seed,
            deck0,
            deck1,
            actions: Vec::new(),
        }
    }

    pub fn push(&mut self, action: Action) {
        self.actions.push(action);
    }

    /// Usado quando o servidor rejeita uma ação: o log não pode divergir
    /// do estado em nenhum caminho.
    pub fn pop(&mut self) {
        self.actions.pop();
    }

    pub fn replay(&self) -> Result<GameState, EngineError> {
        let mut state = new_game(&self.deck0, &self.deck1, self.seed)?;
        for action in &self.actions {
            apply(&mut state, *action)?;
        }
        Ok(state)
    }
}

pub fn replay(rec: &MatchRecord) -> Result<GameState, EngineError> {
    rec.replay()
}
