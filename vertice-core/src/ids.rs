use std::fmt;

pub type PlayerId = u8;
pub type CardId = u16;

/// Identificador de uma *instância* de carta dentro de uma partida.
///
/// Nunca confunda com `CardId`: `CardId` é "qual carta é", `ObjectId` é
/// "esta cópia específica neste jogo". Toda ação, alvo e log usam `ObjectId`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub u32);

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "o{}", self.0)
    }
}
