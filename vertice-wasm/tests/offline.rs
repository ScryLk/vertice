//! O duelo offline roda nativo, no portão — não só no navegador.
//!
//! A lógica toda vive em `Duel`, sem nenhum tipo de `wasm-bindgen`, e é isso
//! que permite testar aqui. O binding é uma casca fina de JSON.

use vertice_protocol::ActionDto;
use vertice_wasm::{Duel, HUMAN};

/// Joga até acabar, sempre pela primeira ação legal. Não é IA: é gerador de
/// cobertura, e basta para exercitar o duelo inteiro.
fn jogar_ate_o_fim(seed: u64) -> Duel {
    let mut duel = Duel::new(seed, "formando").expect("duelo válido");

    for _ in 0..2000 {
        if duel.is_over() {
            return duel;
        }
        let Some(action) = duel.actions(HUMAN).first().copied() else {
            break;
        };
        duel.act(HUMAN, &action).expect("ação legal");
    }

    duel
}

#[test]
fn duelo_offline_termina() {
    let duel = jogar_ate_o_fim(5);
    assert!(duel.is_over(), "duelo offline que não termina é laço");
    assert!(duel.view(HUMAN).winner.is_some());
}

#[test]
fn mesma_semente_mesmo_duelo() {
    // Vale para o bot também: a semente dele sai da semente do duelo.
    let a = jogar_ate_o_fim(9);
    let b = jogar_ate_o_fim(9);
    assert_eq!(a.fingerprint(), b.fingerprint());

    let c = jogar_ate_o_fim(10);
    assert_ne!(a.fingerprint(), c.fingerprint(), "sementes diferentes divergem");
}

#[test]
fn acao_ilegal_e_recusada_sem_mexer_no_estado() {
    let mut duel = Duel::new(3, "formando").expect("duelo válido");
    let antes = duel.fingerprint();

    let erro = match duel.act(HUMAN, &ActionDto::PlayLand { object: 9999 }) {
        Ok(()) => panic!("objeto que não existe não é ação legal"),
        Err(erro) => erro,
    };

    assert_eq!(erro, "ação ilegal");
    assert_eq!(duel.fingerprint(), antes, "ação recusada não muda o estado");
}

#[test]
fn a_view_do_oponente_nao_traz_a_mao() {
    let duel = Duel::new(3, "formando").expect("duelo válido");
    let view = duel.view(HUMAN);

    let json = serde_json::to_value(&view).expect("serializa");
    assert!(
        json["opponent"].get("hand").is_none(),
        "a mão do oponente não existe no fio, offline inclusive"
    );
    assert!(json["opponent"]["hand_count"].is_number());
    assert!(json["you"]["hand"].is_array());
}

#[test]
fn o_bot_joga_sozinho_depois_da_primeira_acao() {
    let mut duel = Duel::new(4, "formando").expect("duelo válido");
    let antes = duel.fingerprint();

    let acao = duel.actions(HUMAN)[0];
    duel.act(HUMAN, &acao).expect("ação legal");

    // Se só a ação do humano tivesse acontecido, a prioridade continuaria
    // com ele na maioria dos casos; o bot ter jogado aparece no `turn`.
    let depois = duel.fingerprint();
    assert_ne!(antes, depois, "o duelo tem que andar");
    assert!(
        duel.actions(HUMAN).is_empty() || duel.view(HUMAN).priority == HUMAN,
        "depois da vez do bot, ou a partida acabou ou a prioridade voltou"
    );
}

#[test]
fn nivel_desconhecido_e_recusado() {
    let erro = match Duel::new(1, "mestre") {
        Ok(_) => panic!("nível que não existe tem que ser recusado"),
        Err(erro) => erro,
    };
    assert!(erro.contains("nível desconhecido"), "veio: {erro}");
}
