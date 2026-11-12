//! Regras da pilha (M1.1).
//!
//! O que estes testes travam:
//!   - mágica conjurada fica na pilha, não no cemitério;
//!   - alvo removido antes da resolução faz a mágica fizzlar;
//!   - abertura roteirizada continua reproduzível bit a bit.
//!
//! Cenários usam `new_game_with_hands` porque mão aleatória não permite
//! testar regra — e setup roteirizado é também o que a campanha vai usar.

use vertice_core::cards::CATALOG;
use vertice_core::*;

/// Busca por nome, nunca por índice: o catálogo vai reordenar no M1.2 e um
/// teste preso a `CardId` numérico quebraria por motivo errado.
fn id(name: &str) -> CardId {
    CATALOG
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("carta {name} não está no catálogo"))
        .id
}

/// Baralho de 100 com as cartas pedidas na frente e o resto em terreno.
fn deck_with(cards: &[CardId]) -> Vec<CardId> {
    let land = id("Várzea de Cruzilha");
    let mut d = cards.to_vec();
    while d.len() < vertice_core::DECK_SIZE {
        d.push(land);
    }
    d
}

fn hand_object(state: &GameState, pid: PlayerId, card: CardId) -> ObjectId {
    state
        .player(pid)
        .zones
        .hand
        .iter()
        .copied()
        .find(|oid| state.object(*oid).card == card)
        .expect("carta deveria estar na mão")
}

fn on_battlefield(state: &GameState, pid: PlayerId, card: CardId) -> ObjectId {
    state
        .player(pid)
        .zones
        .battlefield
        .iter()
        .copied()
        .find(|oid| state.object(*oid).card == card)
        .expect("carta deveria estar no campo")
}

fn zoned(state: &GameState, pid: PlayerId, card: CardId) -> bool {
    state
        .player(pid)
        .zones
        .graveyard
        .iter()
        .any(|oid| state.object(*oid).card == card)
}

struct Ctx {
    state: GameState,
    rec: MatchRecord,
}

impl Ctx {
    fn act(&mut self, action: Action) {
        apply(&mut self.state, action).expect("ação deveria ser legal");
        self.rec.push(action);
    }

    fn play_land(&mut self, pid: PlayerId) {
        let land = id("Várzea de Cruzilha");
        let oid = hand_object(&self.state, pid, land);
        self.act(Action::PlayLand { object: oid });
    }

    fn end_turn(&mut self) {
        self.act(Action::EndTurn);
    }
}

// ---------------------------------------------------------------- testes

#[test]
fn magica_fica_na_pilha_e_so_vai_ao_cemiterio_ao_resolver() {
    let land = id("Várzea de Cruzilha");
    let faisca = id("Faísca");

    let d0 = deck_with(&[faisca]);
    let d1 = deck_with(&[]);
    let h0 = vec![land, faisca];
    let h1 = vec![land];

    let mut ctx = Ctx {
        state: new_game_with_hands(&d0, &d1, 7, &h0, &h1).unwrap(),
        rec: MatchRecord::with_hands(d0, d1, 7, h0, h1),
    };

    // T1 e T2: um terreno cada, mana ainda zero (mana é definida na abertura
    // do turno, e terreno baixado não conta até o turno seguinte).
    ctx.play_land(0);
    ctx.end_turn();
    ctx.play_land(1);
    ctx.end_turn();

    // T3: jogador 0 tem 1 mana.
    assert_eq!(ctx.state.player(0).mana, 1);

    let oid = hand_object(&ctx.state, 0, faisca);
    ctx.act(Action::Cast {
        object: oid,
        target: Some(Target::Player(1)),
    });

    // O que este teste existe para travar:
    assert_eq!(ctx.state.stack.len(), 1, "mágica deve ocupar a pilha");
    assert!(
        !zoned(&ctx.state, 0, faisca),
        "mágica não pode ir ao cemitério antes de resolver"
    );
    assert_eq!(ctx.state.priority, 1, "prioridade passa ao oponente");
    assert_eq!(ctx.state.player(1).life, STARTING_LIFE);

    // Oponente passa: resolve.
    ctx.act(Action::Pass);

    assert_eq!(ctx.state.player(1).life, STARTING_LIFE - 2);
    assert!(zoned(&ctx.state, 0, faisca), "resolveu, agora vai ao cemitério");
    assert!(ctx.state.stack.is_empty());
    assert_eq!(ctx.state.priority, 0, "prioridade volta ao jogador ativo");
}

#[test]
fn alvo_morto_antes_da_resolucao_faz_magica_fizzlar() {
    let land = id("Várzea de Cruzilha");
    let faisca = id("Faísca");
    let sentinela = id("Sentinela de Baluarte");
    let peso = id("Peso Morto");

    let d0 = deck_with(&[faisca]);
    let d1 = deck_with(&[sentinela, peso]);
    let h0 = vec![land, land, faisca];
    let h1 = vec![land, land, sentinela, peso];

    let mut ctx = Ctx {
        state: new_game_with_hands(&d0, &d1, 99, &h0, &h1).unwrap(),
        rec: MatchRecord::with_hands(d0, d1, 99, h0, h1),
    };

    // Terrenos: dois para cada, para haver mana nos turnos seguintes.
    ctx.play_land(0);
    ctx.end_turn();
    ctx.play_land(1);
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.play_land(1);
    ctx.end_turn();
    ctx.end_turn(); // T5: jogador 0 passa sem ação

    // T6: jogador 1 tem 2 mana e baixa a criatura, guardando 1 para a resposta.
    assert_eq!(ctx.state.player(1).mana, 2);
    let s_oid = hand_object(&ctx.state, 1, sentinela);
    ctx.act(Action::Cast {
        object: s_oid,
        target: None,
    });
    assert_eq!(ctx.state.player(1).mana, 1, "reserva de resposta");
    ctx.end_turn();

    // T7: jogador 0 tem 2 mana e mira a criatura.
    assert_eq!(ctx.state.player(0).mana, 2);
    let f_oid = hand_object(&ctx.state, 0, faisca);
    let alvo = on_battlefield(&ctx.state, 1, sentinela);
    ctx.act(Action::Cast {
        object: f_oid,
        target: Some(Target::Object(alvo)),
    });
    assert_eq!(ctx.state.stack.len(), 1);

    // Jogador 1 responde matando a própria criatura com Peso Morto.
    let p_oid = hand_object(&ctx.state, 1, peso);
    ctx.act(Action::Cast {
        object: p_oid,
        target: Some(Target::Object(alvo)),
    });
    assert_eq!(ctx.state.stack.len(), 2, "pilha cheia: só resta Pass");

    // Jogador 0 não pode responder nada além de passar.
    let legais = legal_actions(&ctx.state, 0);
    assert_eq!(legais, vec![Action::Pass]);

    ctx.act(Action::Pass); // resolve Peso Morto: criatura morre
    assert!(
        zoned(&ctx.state, 1, sentinela),
        "criatura deveria ter morrido com -2/-2"
    );

    ctx.act(Action::Pass); // resolve Faísca: alvo não está mais no campo

    assert_eq!(
        ctx.state.player(1).life, STARTING_LIFE,
        "mágica que fizzla não pode causar dano"
    );
    assert!(
        zoned(&ctx.state, 0, faisca),
        "mágica que fizzla ainda vai ao cemitério"
    );
    assert!(ctx.state.stack.is_empty());
}

#[test]
fn replay_de_abertura_roteirizada_reproduz_bit_a_bit() {
    let land = id("Várzea de Cruzilha");
    let faisca = id("Faísca");
    let sentinela = id("Sentinela de Baluarte");
    let peso = id("Peso Morto");

    let mut rec = MatchRecord::with_hands(
        deck_with(&[faisca]),
        deck_with(&[sentinela, peso]),
        123,
        vec![land, land, faisca],
        vec![land, land, sentinela, peso],
    );
    for _ in 0..6 {
        rec.push(Action::EndTurn);
    }

    let a = rec.replay().expect("replay válido");
    let b = rec.replay().expect("replay válido");
    assert_eq!(a, b);
    assert_eq!(a.fingerprint(), b.fingerprint());
}

#[test]
fn pedir_carta_fora_do_baralho_e_erro() {
    let d0 = deck_with(&[]);
    let d1 = deck_with(&[]);
    let faisca = id("Faísca"); // não está em deck_with(&[])
    assert_eq!(
        new_game_with_hands(&d0, &d1, 1, &[faisca], &[]),
        Err(EngineError::BadDeck)
    );
}
