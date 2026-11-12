//! Catálogo de cartas do M0.
//!
//! Neste marco as cartas são `&'static` — o objetivo é provar o motor, não
//! o pipeline de conteúdo. No M1 isto vira dado carregado de arquivo
//! versionado, com a mesma forma.
//!
//! Regra que vale desde já: **carta publicada não é editada.** Muda-se o
//! conjunto, não a carta. Replay de partida antiga depende disso.

use crate::ids::CardId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Keyword {
    /// Só pode ser alvo de ataque por criaturas com voar.
    Flying,
    /// Ataca sem virar.
    Vigilance,
    /// Qualquer dano que cause é letal.
    Deathtouch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardKind {
    Land,
    Creature,
    Spell,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    None,
    DealDamage(u8),
    Draw(u8),
    Pump(i8, i8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CardDef {
    pub id: CardId,
    pub name: &'static str,
    pub cost: u8,
    pub kind: CardKind,
    pub power: i8,
    pub toughness: i8,
    pub keywords: &'static [Keyword],
    pub effect: Effect,
}

// Construtor `const` de carta: argumentos posicionais são o formato mais
// legível para uma tabela de dados, e agrupar em struct só para agradar o
// lint pioraria a leitura de todas as linhas do catálogo.
#[allow(clippy::too_many_arguments)]
const fn def(
    id: CardId,
    name: &'static str,
    cost: u8,
    kind: CardKind,
    power: i8,
    toughness: i8,
    keywords: &'static [Keyword],
    effect: Effect,
) -> CardDef {
    CardDef {
        id,
        name,
        cost,
        kind,
        power,
        toughness,
        keywords,
        effect,
    }
}

pub static CATALOG: &[CardDef] = &[
    def(0, "Várzea de Cruzilha", 0, CardKind::Land, 0, 0, &[], Effect::None),
    def(1, "Enseada Partida", 0, CardKind::Land, 0, 0, &[], Effect::None),
    def(2, "Sentinela de Baluarte", 1, CardKind::Creature, 1, 2, &[Keyword::Vigilance], Effect::None),
    def(3, "Corvo do Arquivo", 2, CardKind::Creature, 2, 1, &[Keyword::Flying], Effect::None),
    def(4, "Cão de Dívida", 2, CardKind::Creature, 2, 2, &[], Effect::None),
    def(5, "Brasa Viva", 1, CardKind::Creature, 2, 1, &[], Effect::None),
    def(6, "Raiz Antiga", 3, CardKind::Creature, 4, 4, &[], Effect::None),
    def(7, "Chancelária", 3, CardKind::Creature, 2, 3, &[], Effect::None),
    def(8, "Vespa do Pântano", 2, CardKind::Creature, 1, 1, &[Keyword::Deathtouch], Effect::None),
    def(9, "Faísca", 1, CardKind::Spell, 0, 0, &[], Effect::DealDamage(2)),
    def(10, "Censo", 2, CardKind::Spell, 0, 0, &[], Effect::Draw(2)),
    def(11, "Peso Morto", 1, CardKind::Spell, 0, 0, &[], Effect::Pump(-2, -2)),
    def(12, "Brasa Dupla", 3, CardKind::Spell, 0, 0, &[], Effect::DealDamage(3)),
];

pub const CATALOG_LEN: usize = CATALOG.len();

pub fn card(id: CardId) -> &'static CardDef {
    let idx = id as usize;
    debug_assert!(idx < CATALOG.len(), "CardId fora do catálogo: {id}");
    &CATALOG[idx]
}
