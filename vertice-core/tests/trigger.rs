//! Gatilho de entrada (M2.1).
//!
//! O que estes testes travam:
//!   - o gatilho de entrada dispara quando a criatura entra, e uma vez só;
//!   - ele resolve **dentro** da conjuração, sem abrir janela de resposta;
//!   - vida ganha como mágica passa pela pilha, como qualquer mágica;
//!   - a partida com gatilho continua reproduzível bit a bit.
//!
//! O cenário usa `new_game_with_hands` pelo mesmo motivo de `stack.rs`: mão
//! aleatória não permite testar regra.

use vertice_core::cards::{CATALOG, Effect};
use vertice_core::*;

fn id(name: &str) -> CardId {
    CATALOG
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("carta {name} não está no catálogo"))
        .id
}

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

fn on_battlefield(state: &GameState, pid: PlayerId, card: CardId) -> bool {
    state
        .player(pid)
        .zones
        .battlefield
        .iter()
        .any(|oid| state.object(*oid).card == card)
}

struct Ctx {
    state: GameState,
    rec: MatchRecord,
}

impl Ctx {
    fn new(h0: Vec<CardId>, h1: Vec<CardId>, seed: u64) -> Self {
        let d0 = deck_with(&h0);
        let d1 = deck_with(&h1);
        Ctx {
            state: new_game_with_hands(&d0, &d1, seed, &h0, &h1).expect("abertura válida"),
            rec: MatchRecord::with_hands(d0, d1, seed, h0, h1),
        }
    }

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
fn criatura_com_gatilho_compra_ao_entrar() {
    let land = id("Várzea de Cruzilha");
    let guia = id("Guia do Pátio"); // custo 2, "ao entrar, compre 1 carta"

    let mut ctx = Ctx::new(vec![land, land, guia], vec![land, land, land], 11);

    // Um terreno por turno. No T5 o jogador 0 tem 2 de mana.
    ctx.play_land(0);
    ctx.end_turn();
    ctx.play_land(1);
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn(); // jogador 1 passa o T4
    assert_eq!(ctx.state.player(0).mana, 2);

    let mao_antes = ctx.state.player(0).zones.hand.len();
    let biblioteca_antes = ctx.state.player(0).zones.library.len();
    let topo = *ctx
        .state
        .player(0)
        .zones
        .library
        .last()
        .expect("biblioteca tem 100 cartas");

    let oid = hand_object(&ctx.state, 0, guia);
    ctx.act(Action::Cast {
        object: oid,
        target: None,
    });

    assert!(on_battlefield(&ctx.state, 0, guia), "a criatura entrou");
    // Saiu uma da mão (a criatura) e entrou uma (a compra): o tamanho não muda,
    // e é a *identidade* da carta que prova que o gatilho disparou.
    assert_eq!(ctx.state.player(0).zones.hand.len(), mao_antes);
    assert_eq!(
        ctx.state.player(0).zones.library.len(),
        biblioteca_antes - 1,
        "o gatilho comprou do topo da biblioteca"
    );
    assert!(
        ctx.state.player(0).zones.hand.contains(&topo),
        "o topo da biblioteca tinha que estar na mão"
    );
    assert_eq!(
        ctx.state.priority, 0,
        "gatilho de entrada não passa prioridade: ele resolve na hora"
    );
    assert!(ctx.state.stack.is_empty(), "gatilho não usa a pilha");
}

#[test]
fn criatura_com_gatilho_ganha_vida_e_so_uma_vez() {
    let land = id("Várzea de Cruzilha");
    let zelador = id("Zelador de Chaves"); // custo 2, "ao entrar, ganhe 2 de vida"

    let mut ctx = Ctx::new(vec![land, land, zelador], vec![land, land, land], 12);

    ctx.play_land(0);
    ctx.end_turn();
    ctx.play_land(1);
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();

    let oid = hand_object(&ctx.state, 0, zelador);
    ctx.act(Action::Cast {
        object: oid,
        target: None,
    });
    assert_eq!(ctx.state.player(0).life, STARTING_LIFE + 2);

    // Três turnos depois, a vida não pode ter subido de novo: o gatilho é de
    // *entrada*, não um efeito contínuo disfarçado.
    ctx.end_turn();
    ctx.end_turn();
    ctx.end_turn();
    assert_eq!(ctx.state.player(0).life, STARTING_LIFE + 2);
}

#[test]
fn vida_ganha_como_magica_passa_pela_pilha() {
    let land = id("Várzea de Cruzilha");
    let folego = id("Fôlego Retomado"); // custo 1, "ganha 2 de vida"

    let mut ctx = Ctx::new(vec![land, folego], vec![land, land], 13);

    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn(); // jogador 1 passa o T2
    assert_eq!(ctx.state.player(0).mana, 1);

    let oid = hand_object(&ctx.state, 0, folego);
    ctx.act(Action::Cast {
        object: oid,
        target: None,
    });

    // Mágica é mágica: ocupa a pilha e não resolve sozinha. É aqui que o
    // gatilho e a mágica se separam — mesmos efeitos, tempos diferentes.
    assert_eq!(ctx.state.stack.len(), 1);
    assert_eq!(ctx.state.player(0).life, STARTING_LIFE);

    ctx.act(Action::Pass);
    assert_eq!(ctx.state.player(0).life, STARTING_LIFE + 2);
    assert!(ctx.state.stack.is_empty());
}

#[test]
fn catalogo_so_marca_gatilho_sem_alvo() {
    // O validador é o portão, mas ele roda em outra crate. Aqui é a garantia
    // que o motor precisa: todo gatilho publicado é resolvível sem escolha.
    for def in CATALOG {
        if let Some(trigger) = def.on_enter {
            assert!(
                !trigger.needs_target(),
                "`{}` tem gatilho com alvo, que o motor não sabe resolver",
                def.name
            );
            assert_ne!(
                trigger,
                Effect::None,
                "`{}` tem gatilho vazio",
                def.name
            );
        }
    }
}

#[test]
fn partida_com_gatilho_reproduz_bit_a_bit() {
    let land = id("Várzea de Cruzilha");
    let guia = id("Guia do Pátio");
    let zelador = id("Zelador de Chaves");

    let d0 = deck_with(&[guia]);
    let d1 = deck_with(&[zelador]);
    let h0 = vec![land, land, guia];
    let h1 = vec![land, land, zelador];

    let mut rec = MatchRecord::with_hands(d0, d1, 21, h0, h1);
    for _ in 0..10 {
        rec.push(Action::EndTurn);
    }

    let a = rec.replay().expect("replay válido");
    let b = rec.replay().expect("replay válido");
    assert_eq!(a, b);
    assert_eq!(a.fingerprint(), b.fingerprint());
}
