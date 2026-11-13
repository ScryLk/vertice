//! A carta fora do motor.
//!
//! O cliente precisa saber o que é uma carta — nome, custo, corpo, efeito —
//! para desenhar a ficha. Isso é dado do catálogo, não estado de partida, e
//! por isso vive aqui e não na `PlayerView`: mostrar a ficha não revela nada
//! que a coleção já não mostre.

use serde::{Deserialize, Serialize};
use vertice_core::cards::{CardDef, CardKind, Effect};
use vertice_core::CardId;

use crate::KeywordDto;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKindDto {
    Land,
    Creature,
    Spell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectDto {
    None,
    DealDamage { amount: u8 },
    Draw { amount: u8 },
    Pump { power: i8, toughness: i8 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDefDto {
    pub id: CardId,
    pub name: String,
    pub cost: u8,
    pub kind: CardKindDto,
    pub power: i8,
    pub toughness: i8,
    pub keywords: Vec<KeywordDto>,
    pub effect: EffectDto,
}

impl From<&CardDef> for CardDefDto {
    fn from(def: &CardDef) -> Self {
        CardDefDto {
            id: def.id,
            name: def.name.to_string(),
            cost: def.cost,
            kind: match def.kind {
                CardKind::Land => CardKindDto::Land,
                CardKind::Creature => CardKindDto::Creature,
                CardKind::Spell => CardKindDto::Spell,
            },
            power: def.power,
            toughness: def.toughness,
            keywords: def.keywords.iter().copied().map(KeywordDto::from).collect(),
            effect: match def.effect {
                Effect::None => EffectDto::None,
                Effect::DealDamage(amount) => EffectDto::DealDamage { amount },
                Effect::Draw(amount) => EffectDto::Draw { amount },
                Effect::Pump(power, toughness) => EffectDto::Pump { power, toughness },
            },
        }
    }
}
