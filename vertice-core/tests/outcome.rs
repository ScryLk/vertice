//! Fim de partida.
//!
//! A regra: quando a ação termina de resolver, se alguém está com vida <= 0, a
//! partida acabou. **Sem empate** — se os dois zerarem ao mesmo tempo, vence
//! quem tinha a prioridade quando a ação foi tomada.
//!
//! Checar no meio da resolução seria mais simples e estaria errado: numa troca
//! de dano, quem recebe primeiro morreria antes de o próprio golpe acontecer,
//! e a ordem interna do motor passaria a decidir partidas.

use vertice_core::cards::CATALOG_LEN;
use vertice_core::*;

fn baralho(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

fn partida(seed: u64) -> GameState {
    new_game(&baralho(0), &baralho(3), seed).expect("partida válida")
}

fn acao_qualquer(state: &GameState) -> Action {
    *legal_actions(state, state.priority)
        .first()
        .expect("há ação legal")
}

#[test]
fn partida_comeca_sem_vencedor() {
    let state = partida(1);
    assert_eq!(state.winner, None);
    assert!(!legal_actions(&state, state.priority).is_empty());
}

#[test]
fn vida_zero_encerra_a_partida() {
    let mut state = partida(1);

    // Zera a vida do oponente e deixa uma ação resolver. O motor só checa o
    // fim depois que a ação termina, então o estado com vida 0 ainda existe.
    state.player_mut(1).life = 0;

    let action = acao_qualquer(&state);
    apply(&mut state, action).expect("ação legal");

    assert_eq!(state.winner, Some(0), "quem zerou perde");
}

#[test]
fn partida_encerrada_nao_aceita_mais_acao() {
    let mut state = partida(1);
    state.player_mut(1).life = 0;

    let action = acao_qualquer(&state);
    apply(&mut state, action).expect("ação legal");
    assert_eq!(state.winner, Some(0));

    assert!(
        legal_actions(&state, 0).is_empty(),
        "partida encerrada não tem ação legal para ninguém"
    );
    assert!(legal_actions(&state, 1).is_empty());
    assert_eq!(
        apply(&mut state, Action::EndTurn),
        Err(EngineError::GameOver),
        "depois do fim, apply recusa com GameOver"
    );
}

#[test]
fn dupla_morte_desempata_pela_prioridade() {
    let mut state = partida(1);
    let prioridade = state.priority;

    // Os dois zerados na mesma ação. Nenhuma carta do conjunto atual causa
    // isso — é o que uma carta de dano aos dois vai produzir —, então o caso
    // é construído à mão.
    state.player_mut(0).life = 0;
    state.player_mut(1).life = 0;

    let action = acao_qualquer(&state);
    apply(&mut state, action).expect("ação legal");

    assert_eq!(
        state.winner,
        Some(prioridade),
        "sem empate: quem tinha a prioridade vence"
    );
}

#[test]
fn desfecho_faz_parte_do_estado() {
    let aberto = partida(1);
    let mut encerrado = aberto.clone();
    encerrado.winner = Some(1);

    assert_ne!(
        aberto.fingerprint(),
        encerrado.fingerprint(),
        "dois estados que discordam sobre o resultado não são o mesmo estado"
    );
}

#[test]
fn partida_real_termina_com_um_vencedor() {
    let mut state = partida(2024);

    // Política: atacar o jogador sempre que der, senão seguir a primeira ação
    // legal. Determinística, e é o suficiente para alguém morrer.
    for _ in 0..5000 {
        let options = legal_actions(&state, state.priority);
        let Some(action) = options
            .iter()
            .copied()
            .find(|a| matches!(a, Action::Attack { target: Target::Player(_), .. }))
            .or_else(|| options.first().copied())
        else {
            break;
        };
        apply(&mut state, action).expect("ação enumerada é legal");
        if state.winner.is_some() {
            break;
        }
    }

    let winner = state.winner.expect("com ataque disponível, a partida termina");
    let loser = state.opponent_of(winner);
    assert!(
        state.player(loser).life <= 0,
        "quem perdeu é quem zerou"
    );
}
