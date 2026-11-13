//! O protocolo só vale se for reversível — e se a forma dele ficar parada.
//!
//! Três coisas são testadas aqui, e cada uma protege um jeito diferente de
//! quebrar:
//!
//! 1. **Toda variante sobrevive.** Ação que se perde na tradução é bug de
//!    rede, não de regra, e aparece só na partida de alguém.
//! 2. **Uma partida inteira sobrevive.** Serializar, reler e rejogar tem que
//!    dar o mesmo fingerprint. É o que faz reconexão e replay serem a mesma
//!    operação.
//! 3. **A forma do JSON fica parada.** Se mudar, não é bug — é aviso de que
//!    `PROTOCOL_VERSION` tem que subir e cliente e servidor têm que subir
//!    juntos.

use vertice_core::cards::{CATALOG, CATALOG_LEN};
use vertice_core::*;
use vertice_protocol::*;

fn baralho(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

/// Partida determinística de verdade. Não é IA: a escolha é `options[step %
/// len]`, que alterna o suficiente para exercitar os caminhos sem depender de
/// nenhum RNG extra.
fn partida(seed: u64) -> MatchRecord {
    let d0 = baralho(0);
    let d1 = baralho(3);
    let mut state = new_game(&d0, &d1, seed).expect("partida válida");
    let mut record = MatchRecord::new(d0, d1, seed);

    for step in 0..300 {
        let options = legal_actions(&state, state.priority);
        if options.is_empty() {
            break;
        }
        let pick = options[step % options.len()];
        apply(&mut state, pick).expect("ação enumerada é legal");
        record.push(pick);
    }

    record
}

#[test]
fn toda_variante_de_acao_vai_e_volta() {
    let acoes = [
        Action::PlayLand {
            object: ObjectId(7),
        },
        Action::Cast {
            object: ObjectId(11),
            target: None,
        },
        Action::Cast {
            object: ObjectId(11),
            target: Some(Target::Player(1)),
        },
        Action::Cast {
            object: ObjectId(11),
            target: Some(Target::Object(ObjectId(42))),
        },
        Action::Attack {
            attacker: ObjectId(3),
            target: Target::Player(0),
        },
        Action::Attack {
            attacker: ObjectId(3),
            target: Target::Object(ObjectId(42)),
        },
        Action::Pass,
        Action::EndTurn,
    ];

    for acao in acoes {
        let json = serde_json::to_string(&ActionDto::from(acao)).expect("serializa");
        let volta: ActionDto = serde_json::from_str(&json).expect("deserializa");
        assert_eq!(
            Action::from(volta),
            acao,
            "ação não sobreviveu à tradução: {json}"
        );
    }
}

#[test]
fn partida_inteira_atravessa_o_json_e_rejoga_igual() {
    let original = partida(7);
    assert!(
        original.actions.len() > 40,
        "precisa de uma partida de verdade, veio {} ações",
        original.actions.len()
    );

    let texto = serde_json::to_string(&MatchRecordDto::from(&original)).expect("serializa");
    let relido: MatchRecordDto = serde_json::from_str(&texto).expect("deserializa");
    let reconstruido = MatchRecord::from(relido);

    assert_eq!(reconstruido, original, "o registro tem que voltar idêntico");

    let antes = original.replay().expect("rejoga original").fingerprint();
    let depois = reconstruido
        .replay()
        .expect("rejoga o relido")
        .fingerprint();
    assert_eq!(
        antes, depois,
        "o registro que atravessou o JSON tem que rejogar o mesmo estado"
    );
}

#[test]
fn forma_json_e_estavel() {
    // Se um destes quebrar, o protocolo mudou. Não é bug: é aviso de que
    // `PROTOCOL_VERSION` sobe e os dois lados sobem junto.
    assert_eq!(
        serde_json::to_string(&ActionDto::from(Action::Pass)).unwrap(),
        r#"{"type":"pass"}"#
    );
    assert_eq!(
        serde_json::to_string(&ActionDto::from(Action::EndTurn)).unwrap(),
        r#"{"type":"end_turn"}"#
    );
    assert_eq!(
        serde_json::to_string(&ActionDto::from(Action::PlayLand {
            object: ObjectId(5),
        }))
        .unwrap(),
        r#"{"type":"play_land","object":5}"#
    );
    assert_eq!(
        serde_json::to_string(&ActionDto::from(Action::Cast {
            object: ObjectId(11),
            target: None,
        }))
        .unwrap(),
        r#"{"type":"cast","object":11,"target":null}"#
    );
    assert_eq!(
        serde_json::to_string(&ActionDto::from(Action::Attack {
            attacker: ObjectId(3),
            target: Target::Object(ObjectId(42)),
        }))
        .unwrap(),
        r#"{"type":"attack","attacker":3,"target":{"type":"object","object":42}}"#
    );
}

#[test]
fn carta_atravessa_o_json_com_o_gatilho() {
    // O catálogo não viaja pela rede — ele é assado no cliente pelo WASM. Mas
    // é por este JSON que a ficha recebe o dado, e um campo que some aqui
    // some da tela em silêncio: a carta perde o texto do gatilho e o sigilo
    // passa a dizer que ela não faz nada ao entrar.
    let com = CATALOG
        .iter()
        .find(|def| def.on_enter.is_some())
        .expect("o conjunto precisa de ao menos uma carta com gatilho");
    let sem = CATALOG
        .iter()
        .find(|def| def.on_enter.is_none())
        .expect("e de ao menos uma sem");

    let dto = CardDefDto::from(com);
    let json = serde_json::to_string(&dto).unwrap();
    assert!(
        json.contains(r#""on_enter":{"#),
        "carta com gatilho tem que mandar o gatilho: {json}"
    );
    // Ausência vai como `null`, não como campo omitido: o cliente não deve
    // precisar distinguir "não tem gatilho" de "o servidor é velho".
    let sem_json = serde_json::to_string(&CardDefDto::from(sem)).unwrap();
    assert!(
        sem_json.contains(r#""on_enter":null"#),
        "carta sem gatilho manda null: {sem_json}"
    );

    let devolta: CardDefDto = serde_json::from_str(&json).unwrap();
    assert_eq!(devolta, dto);
}

#[test]
fn gatilho_de_morte_atravessa_o_json() {
    let com = CATALOG
        .iter()
        .find(|def| def.on_die.is_some())
        .expect("o conjunto precisa de ao menos uma carta com gatilho de morte");
    let json = serde_json::to_string(&CardDefDto::from(com)).unwrap();
    assert!(
        json.contains(r#""on_die":{"#),
        "carta com gatilho de morte tem que mandar o gatilho: {json}"
    );

    let sem = CATALOG
        .iter()
        .find(|def| def.on_enter.is_none() && def.on_die.is_none())
        .expect("e de pelo menos uma sem gatilho nenhum");
    let sem_json = serde_json::to_string(&CardDefDto::from(sem)).unwrap();
    assert!(sem_json.contains(r#""on_enter":null"#), "{sem_json}");
    assert!(sem_json.contains(r#""on_die":null"#), "{sem_json}");
}

#[test]
fn forma_do_efeito_de_vida_e_estavel() {
    assert_eq!(
        serde_json::to_string(&EffectDto::GainLife { amount: 3 }).unwrap(),
        r#"{"kind":"gain_life","amount":3}"#
    );
    assert_eq!(
        serde_json::to_string(&EffectDto::LoseLife { amount: 2 }).unwrap(),
        r#"{"kind":"lose_life","amount":2}"#
    );
    assert_eq!(
        serde_json::to_string(&EffectDto::Destroy).unwrap(),
        r#"{"kind":"destroy"}"#
    );
}

#[test]
fn cliente_consegue_escrever_a_mensagem_a_mao() {
    // O cliente é TypeScript. Ele não tem este crate: ele escreve o JSON.
    // Este teste é o contrato visto do outro lado do fio.
    let json = r#"{"type":"action","seq":7,"action":{"type":"attack","attacker":3,"target":{"type":"player","player":1}}}"#;

    let message: ClientMessage = serde_json::from_str(json).expect("o cliente escreveu isso");
    assert_eq!(
        message,
        ClientMessage::Action {
            seq: 7,
            action: ActionDto::Attack {
                attacker: 3,
                target: TargetDto::Player { player: 1 },
            },
        }
    );
}

#[test]
fn view_do_oponente_leva_contagem_e_nunca_conteudo() {
    let record = partida(7);
    let state = record.replay().expect("rejoga");
    let json = serde_json::to_value(PlayerViewDto::from(&view(&state, 0))).expect("serializa");

    let opponent = &json["opponent"];
    assert!(
        opponent.get("hand").is_none(),
        "a mão do oponente não pode existir no fio, nem vazia"
    );
    assert!(
        opponent["hand_count"].is_number(),
        "do oponente, só a contagem"
    );
    assert!(opponent["library_count"].is_number());

    // A própria mão continua vindo inteira: o jogador pode ver as cartas dele.
    assert!(
        json["you"]["hand"].is_array(),
        "a própria mão tem que vir completa"
    );
    assert!(json["you"]["library_count"].is_number());
}

#[test]
fn a_view_diz_quem_venceu() {
    let record = partida(7);
    let state = record.replay().expect("rejoga");
    let json = serde_json::to_value(PlayerViewDto::from(&view(&state, 0))).expect("serializa");

    assert!(
        json.get("winner").is_some(),
        "sem `winner` o cliente nunca sabe que a partida acabou"
    );
    assert_eq!(
        json["winner"].as_u64(),
        state.winner.map(u64::from),
        "`null` enquanto corre, o assento vencedor quando encerra"
    );
}

#[test]
fn envelope_vai_e_volta() {
    let record = partida(7);
    let state = record.replay().expect("rejoga");
    let welcome = ServerMessage::Welcome {
        protocol: PROTOCOL_VERSION,
        seat: 0,
        seq: 0,
        view: PlayerViewDto::from(&view(&state, 0)),
    };

    let json = serde_json::to_string(&welcome).expect("serializa");
    let volta: ServerMessage = serde_json::from_str(&json).expect("deserializa");
    assert_eq!(volta, welcome);
}
