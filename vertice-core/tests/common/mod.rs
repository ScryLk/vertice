//! Peças compartilhadas entre os testes de integração do core.
//!
//! Fica em subdiretório (`tests/common/mod.rs`) de propósito: arquivo `.rs`
//! solto em `tests/` vira um alvo de teste próprio, e um módulo sem nenhum
//! `#[test]` viraria um binário de teste vazio.

use vertice_core::cards::CATALOG_LEN;
use vertice_core::rng::Rng;
use vertice_core::*;

/// Baralho de 100 cartas ciclando sobre o catálogo. `offset` desloca o ciclo
/// para que dois baralhos não sejam iguais.
pub fn deck(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

/// Joga uma partida inteira com escolhas pseudo-aleatórias determinísticas.
/// Não é IA — é gerador de cobertura.
///
/// Determinístico em `(seed, chaos_seed)`: as ações são escolhidas por um RNG
/// semeado e por `legal_actions`, então a mesma dupla gera sempre a mesma
/// partida — enquanto o motor não mudar de comportamento. É essa propriedade
/// que a fixture dourada usa.
pub fn scripted_match(seed: u64, chaos_seed: u64) -> MatchRecord {
    let d0 = deck(0);
    let d1 = deck(3);
    let mut state = new_game(&d0, &d1, seed).expect("partida válida");
    let mut rec = MatchRecord::new(d0.clone(), d1.clone(), seed);

    let mut chaos = Rng::new(chaos_seed);

    for _ in 0..600 {
        let options = legal_actions(&state, state.priority);
        if options.is_empty() {
            break;
        }
        let pick = options[chaos.range(options.len())];
        apply(&mut state, pick).expect("ação enumerada é legal");
        rec.push(pick);

        if state.turn > 60 {
            break;
        }
    }

    rec
}
