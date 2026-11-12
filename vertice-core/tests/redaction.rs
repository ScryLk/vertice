//! Testes de redação. Estes são testes de segurança, não de regra.
//!
//! Se um deles falhar, o jogo está entregando informação oculta ao cliente
//! e o ranqueado é trapaça esperando acontecer.

use vertice_core::cards::CATALOG_LEN;
use vertice_core::*;

fn deck(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

#[test]
fn mao_do_oponente_e_apenas_contagem() {
    let state = new_game(&deck(0), &deck(5), 99).unwrap();
    let v = view(&state, 0);

    assert_eq!(v.opponent.hand_count, 7);
    assert_eq!(v.you.hand.len(), 7);
}

#[test]
fn nenhum_object_id_da_mao_alheia_vaza() {
    let state = new_game(&deck(0), &deck(5), 99).unwrap();

    for viewer in 0..2u8 {
        let v = view(&state, viewer);
        let hidden: Vec<ObjectId> = state
            .player(state.opponent_of(viewer))
            .zones
            .hand
            .clone();
        let mentioned = v.mentioned_objects();

        for id in hidden {
            assert!(
                !mentioned.contains(&id),
                "view do jogador {viewer} vazou {id:?}, que está na mão do oponente"
            );
        }
    }
}

#[test]
fn biblioteca_do_oponente_e_apenas_contagem() {
    let state = new_game(&deck(0), &deck(5), 77).unwrap();
    let v = view(&state, 0);

    assert_eq!(v.opponent.library_count, state.player(1).zones.library.len());
    assert_eq!(v.you.library_count, state.player(0).zones.library.len());

    // A view não tem campo de conteúdo de biblioteca, por construção.
    // Este teste existe para o dia em que alguém "otimizar" a struct.
    let mentioned = v.mentioned_objects();
    for id in &state.player(0).zones.library {
        assert!(!mentioned.contains(id));
    }
}

#[test]
fn view_so_traz_acoes_do_jogador_com_prioridade() {
    let state = new_game(&deck(0), &deck(5), 3).unwrap();
    assert_eq!(state.priority, 0);

    let v0 = view(&state, 0);
    assert!(!v0.legal_actions.is_empty(), "jogador 0 tem o que fazer");

    let v1 = view(&state, 1);
    assert!(
        v1.legal_actions.is_empty(),
        "quem não tem prioridade não recebe ação legal"
    );
}
