//! A curva de custo do M0.
//!
//! Referência: `.claude/agents/card-content.md` §2 — criatura N/N custa N;
//! criatura com palavra-chave custa +1; mágica de dano N custa N/2 (2 de dano
//! por 1 mana é o teto); compra de 2 custa 2.
//!
//! Duas coisas que aquela referência não escreve em número e que ficam
//! fixadas **aqui, num lugar só**:
//!
//! 1. Criatura assimétrica (2/1, 1/2) usa o **menor** dos dois valores como
//!    base. É a leitura que fecha com as 13 cartas já publicadas sem nenhuma
//!    justificativa; usar o maior rejeitaria Sentinela de Baluarte (1/2 por 1).
//! 2. A banda é de **±1 mana**. Passar disso é "fora da curva", e a carta
//!    então precisa de `justificativa` no próprio arquivo — que é o escape
//!    que a referência já prevê.
//!
//! Mudar a curva é mudar este arquivo. O validador não sabe nada de custo.

use crate::card::{Card, Effect, Kind};

pub const BAND: i32 = 1;

/// Custo de referência para o custo efetivo da carta, ou `None` quando a
/// combinação não tem curva definida (hoje: mágica sem efeito, que é erro
/// estrutural e é pego antes).
pub fn reference(card: &Card) -> Option<i32> {
    match card.kind {
        Kind::Land => Some(0),
        Kind::Creature => {
            let base = i32::from(card.power.min(card.toughness));
            let keyword_surcharge = if card.keywords.is_empty() { 0 } else { 1 };
            Some(base + keyword_surcharge)
        }
        Kind::Spell => match card.effect {
            Effect::None => None,
            // Teto de 2 de dano por 1 mana.
            Effect::DealDamage(amount) => Some(ceil_half(i32::from(amount))),
            // Compra de N custa N.
            Effect::Draw(amount) => Some(i32::from(amount)),
            // Pump é dano em potencial, mesma régua do dano.
            Effect::Pump(power, toughness) => Some(ceil_half(
                i32::from(power).abs().max(i32::from(toughness).abs()),
            )),
        },
    }
}

fn ceil_half(value: i32) -> i32 {
    (value + 1) / 2
}
