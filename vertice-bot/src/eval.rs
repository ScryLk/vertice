//! Quanto vale uma posição, do ponto de vista de um jogador.
//!
//! Isto é o que a `arquitetura.md` §7 chama de "avaliação de tabuleiro por
//! pesos". Não é IA: é uma função que ordena posições. Toda a inteligência do
//! bot vem de aplicar uma ação e perguntar se a posição resultante é melhor.
//!
//! Os pesos são o que se ajusta quando o bot joga mal, e só eles. A estrutura
//! — vida, mão, mana, corpo — segue o que ganha partida neste jogo, e mudar um
//! número aqui muda o comportamento de todo bot de uma vez.

use vertice_core::cards::{card, CardKind, Keyword};
use vertice_core::{GameState, ObjectId, PlayerId};

/// Vitória vale mais que qualquer posição. Somado a nada: vencer encerra a
/// avaliação, então não existe posição de tabuleiro que compense.
const VITORIA: i32 = 1_000_000;

/// Vida vale 2 por ponto: perder vida é o único jeito de perder a partida, e
/// por isso pesa mais que qualquer carta individual.
const PESO_VIDA: i32 = 2;

/// Carta na mão vale 3: é recurso que ainda vai virar corpo ou resposta.
const PESO_MAO: i32 = 3;

pub fn evaluate(state: &GameState, me: PlayerId) -> i32 {
    if let Some(winner) = state.winner {
        return if winner == me { VITORIA } else { -VITORIA };
    }

    let opponent = state.opponent_of(me);
    let mine = state.player(me);
    let theirs = state.player(opponent);

    let mut score = 0;

    score += (mine.life - theirs.life) * PESO_VIDA;
    score += (mine.zones.hand.len() as i32 - theirs.zones.hand.len() as i32) * PESO_MAO;

    // Mana parada é tempo parado. O conjunto atual não tem mana não gasta
    // entre turnos, então sobra é oportunidade perdida.
    score += i32::from(mine.mana);

    score += board_value(state, me) - board_value(state, opponent);

    score
}

fn board_value(state: &GameState, player: PlayerId) -> i32 {
    state
        .player(player)
        .zones
        .battlefield
        .iter()
        .map(|id| permanent_value(state, *id))
        .sum()
}

fn permanent_value(state: &GameState, id: ObjectId) -> i32 {
    let object = state.object(id);
    let def = card(object.card);

    // Terreno não é corpo: o valor dele já está na mana disponível.
    if def.kind != CardKind::Creature {
        return 0;
    }

    let power = i32::from(def.power + object.buff_power);
    let toughness = i32::from(def.toughness + object.buff_toughness);

    let mut value = power + toughness;

    for keyword in def.keywords {
        value += match keyword {
            // Voar decide combate neste motor: quem não voa não acerta.
            Keyword::Flying => 2,
            Keyword::Vigilance => 1,
            // Toque letal transforma qualquer corpo em remoção.
            Keyword::Deathtouch => 3,
        };
    }

    // Carta danificada está a um passo de morrer, e o dano é removido no
    // fim do turno — mas o risco existe enquanto o turno corre.
    value -= i32::from(object.damage) * 2;

    // Virada já atacou: o valor de pressão deste turno foi usado.
    if object.tapped {
        value -= 1;
    }

    value.max(0)
}
