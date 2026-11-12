//! `PlayerView` na forma de fio.
//!
//! **A view é mão única.** O core a monta com `&'static str` e `&'static
//! [Keyword]` vindos do catálogo compilado; do outro lado só existe texto
//! solto. Converter de volta exigiria re-resolver nome a partir do `CardId`,
//! o que é trabalho do cliente, não da rede. Então não existe `From<Dto>` de
//! volta — e isso é uma propriedade, não uma falta.
//!
//! O que a view **não** carrega continua sendo a fronteira de segurança: mão
//! alheia é contagem, biblioteca é contagem, e nada aqui pode mudar isso.

use serde::{Deserialize, Serialize};
use vertice_core::cards::Keyword;
use vertice_core::{CardId, Phase, PlayerId, PlayerView};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeywordDto {
    Flying,
    Vigilance,
    Deathtouch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseDto {
    Main,
    Reaction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardViewDto {
    pub object: u32,
    pub card: CardId,
    pub name: String,
    pub power: i8,
    pub toughness: i8,
    pub tapped: bool,
    pub damage: u8,
    pub keywords: Vec<KeywordDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpponentViewDto {
    pub id: PlayerId,
    pub life: i32,
    /// Contagem, nunca conteúdo.
    pub hand_count: usize,
    pub library_count: usize,
    pub battlefield: Vec<CardViewDto>,
    pub graveyard: Vec<CardId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfViewDto {
    pub id: PlayerId,
    pub life: i32,
    pub hand: Vec<CardViewDto>,
    pub library_count: usize,
    pub battlefield: Vec<CardViewDto>,
    pub graveyard: Vec<CardId>,
    pub mana: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackViewDto {
    pub controller: PlayerId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerViewDto {
    pub you: SelfViewDto,
    pub opponent: OpponentViewDto,
    pub turn: u32,
    pub phase: PhaseDto,
    pub priority: PlayerId,
    pub stack: Vec<StackViewDto>,
    pub legal_actions: Vec<crate::protocol::ActionDto>,
    /// Quem venceu. `null` enquanto a partida corre.
    pub winner: Option<PlayerId>,
}

impl From<Keyword> for KeywordDto {
    fn from(keyword: Keyword) -> Self {
        match keyword {
            Keyword::Flying => KeywordDto::Flying,
            Keyword::Vigilance => KeywordDto::Vigilance,
            Keyword::Deathtouch => KeywordDto::Deathtouch,
        }
    }
}

impl From<Phase> for PhaseDto {
    fn from(phase: Phase) -> Self {
        match phase {
            Phase::Main => PhaseDto::Main,
            Phase::Reaction => PhaseDto::Reaction,
        }
    }
}

fn card_view_dto(card: &vertice_core::view::CardView) -> CardViewDto {
    CardViewDto {
        object: card.object.0,
        card: card.card,
        name: card.name.to_string(),
        power: card.power,
        toughness: card.toughness,
        tapped: card.tapped,
        damage: card.damage,
        keywords: card.keywords.iter().copied().map(KeywordDto::from).collect(),
    }
}

impl From<&PlayerView> for PlayerViewDto {
    fn from(view: &PlayerView) -> Self {
        PlayerViewDto {
            you: SelfViewDto {
                id: view.you.id,
                life: view.you.life,
                hand: view.you.hand.iter().map(card_view_dto).collect(),
                library_count: view.you.library_count,
                battlefield: view.you.battlefield.iter().map(card_view_dto).collect(),
                graveyard: view.you.graveyard.clone(),
                mana: view.you.mana,
            },
            opponent: OpponentViewDto {
                id: view.opponent.id,
                life: view.opponent.life,
                hand_count: view.opponent.hand_count,
                library_count: view.opponent.library_count,
                battlefield: view
                    .opponent
                    .battlefield
                    .iter()
                    .map(card_view_dto)
                    .collect(),
                graveyard: view.opponent.graveyard.clone(),
            },
            turn: view.turn,
            phase: view.phase.into(),
            priority: view.priority,
            stack: view
                .stack
                .iter()
                .map(|entry| StackViewDto {
                    controller: entry.controller,
                    name: entry.name.to_string(),
                })
                .collect(),
            legal_actions: view.legal_actions.iter().copied().map(Into::into).collect(),
            winner: view.winner,
        }
    }
}
