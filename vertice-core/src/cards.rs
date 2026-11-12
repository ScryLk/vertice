//! Catálogo de cartas.
//!
//! As cartas são **dado**, não código: `cards/registry.toml` dá o `CardId`
//! estável e `cards/set-*/*.toml` dá a carta. `vertice-tools generate`
//! compila os dois para `src/generated/cards.rs`, que é **commitado** — quem
//! clona o repositório compila sem rodar ferramenta nenhuma, e o motor
//! continua com zero dependências.
//!
//! Regra que vale desde já e para sempre: **carta publicada não é editada.**
//! Muda-se o conjunto, não a carta. Replay de partida antiga depende disso.
//!
//! Aqui ficam só os tipos da DSL. O conteúdo mora no gerado.

use crate::ids::CardId;

pub use crate::generated::cards::CATALOG;

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

pub const CATALOG_LEN: usize = CATALOG.len();

/// Busca por `CardId`.
///
/// Antes isto era `CATALOG[id as usize]`: o id *era* a posição, então mexer na
/// ordem do arquivo trocava uma carta por outra sem avisar e quebrava replay
/// em silêncio. Agora o id vem do registro e a busca é pela chave — reordenar
/// o catálogo deixou de ser capaz de mudar o significado de um id.
///
/// O catálogo é gerado em ordem crescente de id, o que a busca binária exige;
/// `tests/catalog.rs` guarda essa invariante.
pub fn card(id: CardId) -> &'static CardDef {
    match CATALOG.binary_search_by_key(&id, |def| def.id) {
        Ok(index) => &CATALOG[index],
        Err(_) => panic!("CardId fora do catálogo: {id}"),
    }
}
