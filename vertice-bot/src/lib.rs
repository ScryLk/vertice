//! O bot do Vértice.
//!
//! É um consumidor do motor, não parte dele: só enxerga `legal_actions`,
//! `apply` e o estado. **Nunca decide legalidade** — escolhe de dentro da
//! lista que o motor deu, e é por isso que cliente e servidor nunca discordam
//! sobre o que o bot pode fazer.
//!
//! ## Como ele pensa
//!
//! Um passo à frente: para cada ação legal, aplica num clone do estado e
//! pergunta se a posição resultante é melhor (`eval::evaluate`). Escolhe a
//! melhor. Empate fica com a primeira, e a ordem de `legal_actions` é
//! determinística, então o bot é determinístico.
//!
//! Não é busca, não é Monte Carlo, não é LLM. É a fase 1 da `arquitetura.md`
//! §7: heurística que bate jogador médio o bastante para sustentar a campanha.
//! A fase 2 — busca com simulações — só faz sentido depois que o motor estiver
//! estável, e o motor acabou de estabilizar.
//!
//! ## O que ele enxerga
//!
//! **O estado inteiro, inclusive a mão do oponente.** É IA de servidor, não
//! cliente: a redação existe para impedir jogador de trapacear, não para
//! cegar o NPC. Se um dia o bot for rodar no cliente, isso muda.

use vertice_core::rng::Rng;
use vertice_core::{apply, legal_actions, Action, GameState, PlayerId};

pub mod eval;

/// Dificuldade. A diferença entre elas é **só** a frequência de erro
/// deliberado — um calouro não joga com pesos piores, ele escolhe mal às
/// vezes. É o que a `arquitetura.md` §7 pede: o jogador tem que *sentir* a
/// diferença, não ser informado dela.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    /// Erra de propósito, com frequência.
    Calouro,
    /// Joga sério.
    Formando,
}

impl Level {
    /// Chance, em mil, de escolher ao acaso em vez de avaliar.
    pub fn mistake_per_mille(self) -> usize {
        match self {
            Level::Calouro => 350,
            Level::Formando => 0,
        }
    }
}

/// Escolhe uma ação para `player`, ou `None` se não houver nenhuma legal.
///
/// `rng` é injetado, como manda a regra do projeto: nada de entropia
/// ambiente aqui dentro. O mesmo estado, o mesmo nível e a mesma semente
/// produzem sempre a mesma jogada.
pub fn choose_action(
    state: &GameState,
    player: PlayerId,
    level: Level,
    rng: &mut Rng,
) -> Option<Action> {
    let options = legal_actions(state, player);
    if options.is_empty() {
        return None;
    }

    // Erro deliberado. Não consome RNG quando a chance é zero, para que o
    // nível sério seja determinístico mesmo sem semente útil.
    let mistake = level.mistake_per_mille();
    if mistake > 0 && rng.range(1000) < mistake {
        return Some(options[rng.range(options.len())]);
    }

    let mut best = options[0];
    let mut best_score = i32::MIN;

    for action in options {
        let mut next = state.clone();
        if apply(&mut next, action).is_err() {
            // `legal_actions` é o único validador de legalidade. Se aplicar o
            // que ele devolve falha, é bug do motor — e o bot prefere pular a
            // ação a entrar em pânico no meio de uma partida.
            continue;
        }

        let score = eval::evaluate(&next, player);
        if score > best_score {
            best_score = score;
            best = action;
        }
    }

    Some(best)
}
