//! O teste que justifica a arquitetura inteira.
//!
//! Se este arquivo ficar verde, você pode refatorar o motor por um ano
//! sem medo. Se ficar vermelho, você quebrou compatibilidade — e você
//! descobre em segundos, não em produção.

use vertice_core::cards::CATALOG_LEN;
use vertice_core::rng::Rng;
use vertice_core::*;

fn deck(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

/// Joga uma partida inteira com escolhas pseudo-aleatórias determinísticas.
/// Não é IA — é gerador de cobertura.
fn scripted_match(seed: u64, chaos_seed: u64) -> MatchRecord {
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

#[test]
fn mesmo_seed_mesmo_estado() {
    let a = new_game(&deck(0), &deck(3), 42).unwrap();
    let b = new_game(&deck(0), &deck(3), 42).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.fingerprint(), b.fingerprint());
}

#[test]
fn seeds_diferentes_embaralham_diferente() {
    let a = new_game(&deck(0), &deck(3), 42).unwrap();
    let b = new_game(&deck(0), &deck(3), 43).unwrap();
    assert_ne!(
        a.player(0).zones.library,
        b.player(0).zones.library,
        "shuffle não está usando a seed"
    );
}

#[test]
fn replay_reproduz_bit_a_bit() {
    let rec = scripted_match(1234, 99);
    assert!(
        rec.actions.len() > 40,
        "o gerador precisa produzir uma partida de verdade, veio {}",
        rec.actions.len()
    );

    let original = {
        let mut state = new_game(&rec.deck0, &rec.deck1, rec.seed).unwrap();
        for a in &rec.actions {
            apply(&mut state, *a).unwrap();
        }
        state
    };

    let replayed = replay(&rec).expect("replay não pode falhar");

    assert_eq!(original, replayed);
    assert_eq!(original.fingerprint(), replayed.fingerprint());
}

#[test]
fn replay_dez_vezes_da_o_mesmo_resultado() {
    let rec = scripted_match(7, 11);
    let first = replay(&rec).unwrap();
    for _ in 0..10 {
        assert_eq!(replay(&rec).unwrap(), first);
    }
}

#[test]
fn acao_ilegal_e_recusada_sem_sujar_o_estado() {
    let mut state = new_game(&deck(0), &deck(3), 5).unwrap();
    let before = state.fingerprint();

    // Oponente não tem prioridade no turno 1.
    let err = apply(
        &mut state,
        Action::PlayLand {
            object: ObjectId(9999),
        },
    );
    assert_eq!(err, Err(EngineError::IllegalAction));
    assert_eq!(state.fingerprint(), before, "estado não pode mudar");
    assert!(state.log.is_empty(), "log não pode registrar ação recusada");
}

#[test]
fn deck_fora_do_tamanho_e_recusado() {
    assert_eq!(
        new_game(&[0u16; 40], &deck(0), 1),
        Err(EngineError::BadDeck)
    );
}
