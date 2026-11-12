//! `MatchRecord` na forma de armazenamento.
//!
//! Uma partida inteira é isto: versões, seed, dois baralhos, mãos iniciais e
//! a lista de ações. Nada de estados intermediários. Gravar isso e rejogar é
//! o que permite reconexão, espectador e replay público sem mais nenhuma
//! infraestrutura.
//!
//! Este é o tipo que vai e volta do disco. Diferente da view, ele **é**
//! reversível: dá para reconstruir a partida a partir dele, e o teste faz
//! exatamente isso — grava, relê, rejoga e compara o fingerprint.

use serde::{Deserialize, Serialize};
use vertice_core::{CardId, MatchRecord};

use super::ActionDto;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRecordDto {
    pub engine_version: String,
    pub card_set_version: String,
    pub seed: u64,
    pub deck0: Vec<CardId>,
    pub deck1: Vec<CardId>,
    pub hand0: Vec<CardId>,
    pub hand1: Vec<CardId>,
    pub actions: Vec<ActionDto>,
}

impl From<&MatchRecord> for MatchRecordDto {
    fn from(record: &MatchRecord) -> Self {
        MatchRecordDto {
            engine_version: record.engine_version.clone(),
            card_set_version: record.card_set_version.clone(),
            seed: record.seed,
            deck0: record.deck0.clone(),
            deck1: record.deck1.clone(),
            hand0: record.hand0.clone(),
            hand1: record.hand1.clone(),
            actions: record.actions.iter().copied().map(ActionDto::from).collect(),
        }
    }
}

impl From<MatchRecordDto> for MatchRecord {
    fn from(record: MatchRecordDto) -> Self {
        MatchRecord {
            engine_version: record.engine_version,
            card_set_version: record.card_set_version,
            seed: record.seed,
            deck0: record.deck0,
            deck1: record.deck1,
            hand0: record.hand0,
            hand1: record.hand1,
            actions: record.actions.into_iter().map(Into::into).collect(),
        }
    }
}
