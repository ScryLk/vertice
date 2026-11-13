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
    GainLife(u8),
    /// O oponente do controlador perde N de vida.
    ///
    /// Sem alvo, e é por isso que existe: "cada oponente perde 2" não é o
    /// mesmo que "causa 2 de dano a qualquer alvo", porque o segundo obriga a
    /// escolher. Num gatilho, que não abre janela de resposta, a diferença é a
    /// carta existir ou não.
    LoseLife(u8),
}

impl Effect {
    /// O efeito exige que o jogador escolha um alvo?
    ///
    /// Existe uma definição só porque há dois consumidores com interesse
    /// oposto: o motor pergunta para decidir se a conjuração precisa carregar
    /// alvo, e o validador pergunta para recusar efeito com alvo em gatilho —
    /// gatilho com alvo exigiria escolha no meio da resolução, e escolha exige
    /// janela na pilha, que gatilho ainda não tem. Duas cópias divergiriam.
    pub fn needs_target(self) -> bool {
        matches!(self, Effect::DealDamage(_) | Effect::Pump(..))
    }
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
    /// Efeito que dispara quando a criatura entra no campo de batalha.
    ///
    /// Só criatura tem, e só efeito **sem alvo** cabe aqui. O gatilho resolve
    /// na hora, dentro da própria conjuração: sem alvo não há escolha, e sem
    /// escolha não há o que a janela de resposta proteger. Gatilho com alvo é
    /// fatia própria, e depende de gatilho usar a pilha.
    pub on_enter: Option<Effect>,
    /// Efeito que dispara quando a criatura **morre** — isto é, quando
    /// `cleanup` a move do campo para o cemitério. Sair do campo por outro
    /// caminho não é morrer.
    ///
    /// Mesmo corte do `on_enter`: só efeito sem alvo. A diferença é o momento,
    /// e é ela que dá sentido a trocar criatura — sem gatilho de morte,
    /// atacar em troca é sempre perder.
    pub on_die: Option<Effect>,
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

/// A carta existe no catálogo?
///
/// O motor recebe `CardId` de fora — baralho do cliente, `MatchRecord` lido
/// do banco. `card()` entra em pânico com id desconhecido, e isso é
/// deliberado: id inválido no meio da partida é bug, não erro de entrada.
/// Esta checagem existe para que a entrada seja recusada na porta, e o
/// pânico deixe de ser alcançável por dado de fora.
pub(crate) fn exists(id: CardId) -> bool {
    CATALOG.binary_search_by_key(&id, |def| def.id).is_ok()
}

/// Baralho provisório: `DECK_SIZE` cartas ciclando o catálogo.
///
/// **Placeholder declarado** até deck building existir. Vive aqui porque o
/// servidor e o duelo offline precisam do mesmo baralho, e duas cópias
/// divergiriam — o teste do bot já mostrou que até o *deslocamento* muda a
/// composição: 100 cartas em 13 valores deixam 9 valores com 8 cópias e 4 com
/// 7, e quais são depende do offset.
pub fn provisional_deck(offset: u16) -> Vec<CardId> {
    let catalogo = CATALOG_LEN as u16;
    (0..crate::DECK_SIZE as u16)
        .map(|i| i.wrapping_mul(7).wrapping_add(offset) % catalogo)
        .collect()
}
