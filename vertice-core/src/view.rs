//! Redação: a única forma de o cliente ver o estado.
//!
//! A função `view` é o contrato de segurança do jogo. Ela nunca devolve a
//! mão do oponente, nunca devolve a ordem da biblioteca, e nunca devolve a
//! identidade de uma carta virada pra baixo. Cliente que recebe a view
//! inteira é cliente que pode trapacear.
//!
//! Regra de ouro: o cliente não recebe o `GameState`. Nem pra "debugar".

use crate::action::Action;
use crate::cards::{card, Keyword};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameState, Phase};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardView {
    pub object: ObjectId,
    pub card: CardId,
    pub name: &'static str,
    pub power: i8,
    pub toughness: i8,
    pub tapped: bool,
    pub damage: u8,
    pub keywords: &'static [Keyword],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpponentView {
    pub id: PlayerId,
    pub life: i32,
    /// Contagem, nunca conteúdo.
    pub hand_count: usize,
    pub library_count: usize,
    pub battlefield: Vec<CardView>,
    pub graveyard: Vec<CardId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfView {
    pub id: PlayerId,
    pub life: i32,
    pub hand: Vec<CardView>,
    pub library_count: usize,
    pub battlefield: Vec<CardView>,
    pub graveyard: Vec<CardId>,
    pub mana: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackView {
    pub controller: PlayerId,
    pub name: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerView {
    pub you: SelfView,
    pub opponent: OpponentView,
    pub turn: u32,
    pub phase: Phase,
    pub priority: PlayerId,
    pub stack: Vec<StackView>,
    pub legal_actions: Vec<Action>,
}

impl PlayerView {
    /// Todo `ObjectId` que esta view revela. Existe para o teste de redação
    /// e para auditoria manual: qualquer objeto fora desta lista é segredo.
    pub fn mentioned_objects(&self) -> Vec<ObjectId> {
        let mut v = Vec::new();
        v.extend(self.you.hand.iter().map(|c| c.object));
        v.extend(self.you.battlefield.iter().map(|c| c.object));
        v.extend(self.opponent.battlefield.iter().map(|c| c.object));
        v
    }
}

fn card_view(state: &GameState, id: ObjectId) -> CardView {
    let o = state.object(id);
    let def = card(o.card);
    CardView {
        object: id,
        card: o.card,
        name: def.name,
        power: def.power + o.buff_power,
        toughness: def.toughness + o.buff_toughness,
        tapped: o.tapped,
        damage: o.damage,
        keywords: def.keywords,
    }
}

fn zone_views(state: &GameState, ids: &[ObjectId]) -> Vec<CardView> {
    ids.iter().map(|id| card_view(state, *id)).collect()
}

fn graveyard_cards(state: &GameState, ids: &[ObjectId]) -> Vec<CardId> {
    ids.iter().map(|id| state.object(*id).card).collect()
}

/// Monta a view de `viewer`. Nada além do que ele pode ver.
pub fn view(state: &GameState, viewer: PlayerId) -> PlayerView {
    let me = state.player(viewer);
    let opp = state.player(state.opponent_of(viewer));

    PlayerView {
        you: SelfView {
            id: me.id,
            life: me.life,
            hand: zone_views(state, &me.zones.hand),
            library_count: me.zones.library.len(),
            battlefield: zone_views(state, &me.zones.battlefield),
            graveyard: graveyard_cards(state, &me.zones.graveyard),
            mana: me.mana,
        },
        opponent: OpponentView {
            id: opp.id,
            life: opp.life,
            hand_count: opp.zones.hand.len(),
            library_count: opp.zones.library.len(),
            battlefield: zone_views(state, &opp.zones.battlefield),
            graveyard: graveyard_cards(state, &opp.zones.graveyard),
        },
        turn: state.turn,
        phase: state.phase,
        priority: state.priority,
        stack: state
            .stack
            .iter()
            .map(|p| StackView {
                controller: p.controller,
                name: card(state.object(p.object).card).name,
            })
            .collect(),
        legal_actions: crate::engine::legal_actions(state, viewer),
    }
}
