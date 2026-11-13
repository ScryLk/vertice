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

    /// Avança `terrenos` turnos do jogador 0, baixando um terreno em cada um.
    ///
    /// Termina no turno do jogador 0 com `terrenos` de mana: a mana é definida
    /// na abertura do turno, então o terreno baixado no último turno só conta
    /// no seguinte — e é por isso que o laço termina um turno depois.
    fn avancar(&mut self, terrenos: usize) {
        for _ in 0..terrenos {
            self.play_land(0);
            self.end_turn();
            self.end_turn();
        }
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
fn criatura_com_gatilho_de_morte_compra_ao_morrer() {
    let land = id("Várzea de Cruzilha");
    let sinaleiro = id("Sinaleiro do Arquivo"); // 2/2, "ao morrer, compre 1"
    let faisca = id("Faísca"); // 2 de dano a qualquer alvo

    // Quatro terrenos para ter 4 de mana no sétimo turno: 3 pela criatura,
    // 1 pela mágica que a mata.
    let mut ctx = Ctx::new(
        vec![land, land, land, land, sinaleiro, faisca],
        vec![land],
        14,
    );

    // Mana vem dos terrenos que já estão no campo quando o turno abre, então
    // o quarto terreno só vira mana no turno seguinte: quatro terrenos ao fim
    // do T7 dão 4 de mana no T9.
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    assert_eq!(ctx.state.player(0).mana, 4);

    let s_oid = hand_object(&ctx.state, 0, sinaleiro);
    ctx.act(Action::Cast {
        object: s_oid,
        target: None,
    });
    assert!(on_battlefield(&ctx.state, 0, sinaleiro));

    let biblioteca_antes = ctx.state.player(0).zones.library.len();
    let f_oid = hand_object(&ctx.state, 0, faisca);
    ctx.act(Action::Cast {
        object: f_oid,
        target: Some(Target::Object(s_oid)),
    });
    ctx.act(Action::Pass); // jogador 1 passa: a Faísca resolve

    assert!(
        ctx.state.player(0).zones.graveyard.contains(&s_oid),
        "a criatura tinha que ter morrido com 2 de dano"
    );
    assert_eq!(
        ctx.state.player(0).zones.library.len(),
        biblioteca_antes - 1,
        "o gatilho de morte tinha que ter comprado uma carta"
    );
    assert_eq!(
        ctx.state.player(0).life, STARTING_LIFE,
        "este gatilho compra, não mexe em vida"
    );
}

#[test]
fn gatilho_de_morte_nao_dispara_so_por_entrar() {
    let land = id("Várzea de Cruzilha");
    let sinaleiro = id("Sinaleiro do Arquivo");

    let mut ctx = Ctx::new(vec![land, land, land, sinaleiro], vec![land, land], 15);

    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    assert_eq!(ctx.state.player(0).mana, 3);

    let oid = hand_object(&ctx.state, 0, sinaleiro);
    let biblioteca_antes = ctx.state.player(0).zones.library.len();
    ctx.act(Action::Cast {
        object: oid,
        target: None,
    });

    // Entrar não é morrer. Sem esta checagem, um `fire_on_die` no lugar
    // errado passaria despercebido, porque a criatura viva também mudaria
    // alguma coisa em algum teste.
    assert_eq!(
        ctx.state.player(0).zones.library.len(),
        biblioteca_antes,
        "o gatilho de morte não pode disparar na entrada"
    );
}

#[test]
fn oponente_perde_vida_como_magica() {
    let land = id("Várzea de Cruzilha");
    let cobranca = id("Cobrança de Baluarte"); // custo 1, oponente perde 2

    let mut ctx = Ctx::new(vec![land, cobranca], vec![land, land], 16);

    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    assert_eq!(ctx.state.player(0).mana, 1);

    let oid = hand_object(&ctx.state, 0, cobranca);
    ctx.act(Action::Cast {
        object: oid,
        target: None,
    });

    // Sem alvo, mas continua sendo mágica: ocupa a pilha.
    assert_eq!(ctx.state.stack.len(), 1);
    assert_eq!(ctx.state.player(1).life, STARTING_LIFE);

    ctx.act(Action::Pass);
    assert_eq!(ctx.state.player(1).life, STARTING_LIFE - 2);
    assert_eq!(
        ctx.state.player(0).life, STARTING_LIFE,
        "quem perde vida é o oponente, não o controlador"
    );
}

#[test]
fn gatilho_de_entrada_com_alvo_atinge_o_alvo_escolhido() {
    let land = id("Várzea de Cruzilha");
    let arqueiro = id("Arqueiro do Pátio"); // custo 2, "ao entrar, cause 2 a um alvo"

    let mut ctx = Ctx::new(vec![land, land, arqueiro], vec![land, land], 17);

    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    assert_eq!(ctx.state.player(0).mana, 2);

    let oid = hand_object(&ctx.state, 0, arqueiro);
    ctx.act(Action::Cast {
        object: oid,
        target: Some(Target::Player(1)),
    });

    assert!(on_battlefield(&ctx.state, 0, arqueiro));
    assert_eq!(
        ctx.state.player(1).life,
        STARTING_LIFE - 2,
        "o alvo escolhido na conjuração é quem recebe o gatilho"
    );
}

#[test]
fn gatilho_com_alvo_recusa_conjuracao_sem_alvo() {
    let land = id("Várzea de Cruzilha");
    let arqueiro = id("Arqueiro do Pátio");

    let mut ctx = Ctx::new(vec![land, land, arqueiro], vec![land, land], 18);

    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();

    let oid = hand_object(&ctx.state, 0, arqueiro);
    assert_eq!(
        apply(
            &mut ctx.state,
            Action::Cast {
                object: oid,
                target: None,
            }
        ),
        Err(EngineError::IllegalAction),
        "gatilho com alvo não pode ser conjurado sem escolher o alvo"
    );
}

#[test]
fn gatilho_de_entrada_pode_matar_e_acordar_o_gatilho_de_morte() {
    // A cascata inteira numa ação só: a criatura entra, o gatilho dela mata
    // outra criatura, e a morte dispara o gatilho de morte. Se a ordem interna
    // do motor estivesse errada, a vida não subiria.
    let land = id("Várzea de Cruzilha");
    let zelador = id("Zelador de Cinzas"); // 1/1, custo 2, "ao morrer, ganhe 3"
    let arqueiro = id("Arqueiro do Pátio"); // custo 2, "ao entrar, cause 2 a um alvo"

    let mut ctx = Ctx::new(
        vec![land, land, land, land, zelador, arqueiro],
        vec![land],
        19,
    );

    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    ctx.play_land(0);
    ctx.end_turn();
    ctx.end_turn();
    assert_eq!(ctx.state.player(0).mana, 4);

    let z_oid = hand_object(&ctx.state, 0, zelador);
    ctx.act(Action::Cast {
        object: z_oid,
        target: None,
    });
    assert!(on_battlefield(&ctx.state, 0, zelador));

    let a_oid = hand_object(&ctx.state, 0, arqueiro);
    ctx.act(Action::Cast {
        object: a_oid,
        target: Some(Target::Object(z_oid)),
    });

    assert!(
        ctx.state.player(0).zones.graveyard.contains(&z_oid),
        "o gatilho de entrada tinha que matar o Zelador (1/1 contra 2 de dano)"
    );
    assert_eq!(
        ctx.state.player(0).life,
        STARTING_LIFE + 3,
        "e a morte dele tinha que disparar o gatilho de morte"
    );
}

#[test]
fn gatilho_de_entrada_destroi_a_criatura_alvo() {
    let land = id("Várzea de Cruzilha");
    let brasa = id("Brasa Viva"); // 2/1, custo 1
    let carrasco = id("Carrasco do Poço"); // custo 4, "ao entrar, destrua a criatura alvo"

    let mut ctx = Ctx::new(vec![land, land, land, land, land, brasa, carrasco], vec![land], 20);
    ctx.avancar(5);
    assert_eq!(ctx.state.player(0).mana, 5);

    let b_oid = hand_object(&ctx.state, 0, brasa);
    ctx.act(Action::Cast {
        object: b_oid,
        target: None,
    });

    let c_oid = hand_object(&ctx.state, 0, carrasco);
    ctx.act(Action::Cast {
        object: c_oid,
        target: Some(Target::Object(b_oid)),
    });

    // Destruir não pergunta resistência: 2/1 morre igual, e sai do campo antes
    // de qualquer conta de dano.
    assert!(
        ctx.state.player(0).zones.graveyard.contains(&b_oid),
        "a criatura destruída tinha que estar no cemitério"
    );
    assert!(!on_battlefield(&ctx.state, 0, brasa));
    assert!(on_battlefield(&ctx.state, 0, carrasco));
}

#[test]
fn catalogo_nao_pede_alvo_onde_o_motor_nao_sabe_escolher() {
    // O validador é o portão, mas ele roda em outra crate. Aqui é a garantia
    // que o motor precisa: gatilho de morte é sempre sem alvo, porque não
    // existe ação do controlador para carregar a escolha.
    for def in CATALOG {
        if let Some(effect) = def.on_die {
            assert!(
                !effect.needs_target(),
                "`{}` tem gatilho de morte com alvo, que o motor não sabe resolver",
                def.name
            );
            assert_ne!(effect, Effect::None, "`{}` tem `on_die` vazio", def.name);
        }
        if let Some(effect) = def.on_enter {
            assert_ne!(effect, Effect::None, "`{}` tem `on_enter` vazio", def.name);
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
