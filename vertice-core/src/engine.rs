//! Regras de transição de estado.
//!
//! Simplificações conscientes do motor, todas reversíveis:
//!
//! - **Mana automática.** O pool do turno é igual ao número de terrenos no
//!   início do turno. Terrenos não viram. No motor real isso vira
//!   ativação de terreno.
//! - **Sem etapa de bloqueio.** Atacar criatura = troca de dano direta.
//! - **Pilha de no máximo 2.** Uma mágica + uma resposta. É a "janela de
//!   reação única" do documento de regras, implementada literalmente.
//! - **Criaturas não usam a pilha.** Só mágicas abrem janela de resposta.
//! - **A mágica ocupa a pilha de verdade (M1.1).** Ela sai da mão, fica na
//!   pilha e só vai ao cemitério quando resolve. Se o alvo deixar o campo
//!   antes disso, ela **fizzla**: não faz nada, mas continua tendo sido
//!   conjurada e paga.
//!
//! Nenhuma dessas simplificações pode violar as invariantes do crate:
//! determinismo, redação e autoridade.

use crate::action::Action;
use crate::cards::{card, CardKind, Effect, Keyword};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameState, Object, Phase, PendingSpell, Player, Target, Zones};
use crate::rng::Rng;
use crate::{DECK_SIZE, OPENING_HAND, STARTING_LIFE};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EngineError {
    IllegalAction,
    BadDeck,
    GameOver,
}

/// Cria uma partida. Determinístico em `(deck0, deck1, seed)`.
pub fn new_game(deck0: &[CardId], deck1: &[CardId], seed: u64) -> Result<GameState, EngineError> {
    if deck0.len() != DECK_SIZE || deck1.len() != DECK_SIZE {
        return Err(EngineError::BadDeck);
    }

    let mut objects = std::collections::BTreeMap::new();
    let mut next: u32 = 0;
    let mut players = Vec::with_capacity(2);

    for (i, deck) in [deck0, deck1].into_iter().enumerate() {
        let pid = i as PlayerId;
        let mut library = Vec::with_capacity(deck.len());
        for &cid in deck {
            let id = ObjectId(next);
            next += 1;
            objects.insert(id, Object::new(id, cid, pid));
            library.push(id);
        }
        players.push(Player {
            id: pid,
            name: if pid == 0 { "Você" } else { "Oponente" },
            life: STARTING_LIFE,
            zones: Zones {
                library,
                ..Default::default()
            },
            mana: 0,
            lands_played: 0,
        });
    }

    let mut state = GameState {
        players,
        objects,
        next_object: next,
        active: 0,
        priority: 0,
        turn: 0,
        phase: Phase::Main,
        stack: Vec::new(),
        rng: Rng::new(seed),
        log: Vec::new(),
    };

    for pid in 0..2u8 {
        shuffle_library(&mut state, pid);
        draw(&mut state, pid, OPENING_HAND);
    }

    // Primeiro turno não compra.
    begin_turn(&mut state, 0, false);
    Ok(state)
}

/// Cria uma partida com **mãos iniciais forçadas**, retirando essas cartas da
/// biblioteca já embaralhada.
///
/// Existe para dois usos legítimos: teste de regra e duelo roteirizado da
/// campanha (NPC com abertura fixa). Como a biblioteca é embaralhada pela
/// seed e a retirada é determinística, `new_game_with_hands` com os mesmos
/// argumentos produz exatamente o mesmo estado — é o que permite o
/// `MatchRecord` guardar as mãos e o replay continuar fiel.
///
/// Erro `BadDeck` se a carta pedida não estiver no baralho.
pub fn new_game_with_hands(
    deck0: &[CardId],
    deck1: &[CardId],
    seed: u64,
    hand0: &[CardId],
    hand1: &[CardId],
) -> Result<GameState, EngineError> {
    let mut state = new_game(deck0, deck1, seed)?;

    for (pid, wanted) in [(0u8, hand0), (1u8, hand1)] {
        // Devolve a mão sorteada para a biblioteca e extrai exatamente o que
        // foi pedido. A ordem final é determinística porque parte do
        // embaralhamento da seed e não reembaralha.
        let drawn = std::mem::take(&mut state.player_mut(pid).zones.hand);
        state.player_mut(pid).zones.library.extend(drawn);

        for &cid in wanted {
            let pos = state
                .player(pid)
                .zones
                .library
                .iter()
                .rposition(|oid| state.object(*oid).card == cid)
                .ok_or(EngineError::BadDeck)?;
            let oid = state.player_mut(pid).zones.library.remove(pos);
            state.player_mut(pid).zones.hand.push(oid);
        }
    }

    // O turno já foi aberto em `new_game`; a mana veio de campo vazio (zero),
    // que continua sendo o correto aqui.
    Ok(state)
}

/// Enumera as ações legais do jogador. É o único validador de legalidade —
/// `apply` aceita exatamente o que isto devolve.
pub fn legal_actions(state: &GameState, pid: PlayerId) -> Vec<Action> {
    let mut out = Vec::new();
    if pid != state.priority {
        return out;
    }

    match state.phase {
        Phase::Reaction => {
            out.extend(candidate_casts(state, pid));
            out.push(Action::Pass);
        }
        Phase::Main => {
            let lands_played = state.player(pid).lands_played;
            if lands_played < 1 {
                for &oid in &state.player(pid).zones.hand {
                    if card(state.object(oid).card).kind == CardKind::Land {
                        out.push(Action::PlayLand { object: oid });
                    }
                }
            }

            out.extend(candidate_casts(state, pid));

            let battlefield = state.player(pid).zones.battlefield.clone();
            for oid in battlefield {
                let o = state.object(oid);
                let def = card(o.card);
                if def.kind == CardKind::Creature && !o.tapped && !o.summoned {
                    for target in attack_targets(state, pid, oid) {
                        out.push(Action::Attack {
                            attacker: oid,
                            target,
                        });
                    }
                }
            }

            out.push(Action::EndTurn);
        }
    }

    out
}

pub fn apply(state: &mut GameState, action: Action) -> Result<(), EngineError> {
    if !legal_actions(state, state.priority).contains(&action) {
        return Err(EngineError::IllegalAction);
    }

    // O log é a fonte de verdade do replay. Registre antes de resolver:
    // se a resolução entrar em pânico, o log não mente sobre a intenção.
    state.log.push(action);

    match action {
        Action::PlayLand { object } => {
            let pid = state.object(object).controller;
            state.player_mut(pid).zones.hand.retain(|x| *x != object);
            state.player_mut(pid).zones.battlefield.push(object);
            state.player_mut(pid).lands_played += 1;
        }
        Action::Cast { object, target } => cast(state, object, target)?,
        Action::Attack { attacker, target } => attack(state, attacker, target)?,
        Action::Pass => pass(state)?,
        Action::EndTurn => {
            let next = state.opponent_of(state.active);
            begin_turn(state, next, true);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------- internos

fn cast(state: &mut GameState, object: ObjectId, target: Option<Target>) -> Result<(), EngineError> {
    let obj = *state.object(object);
    let def = card(obj.card);
    let pid = obj.controller;

    let remaining = state.player(pid).mana.saturating_sub(def.cost);
    state.player_mut(pid).mana = remaining;
    state.player_mut(pid).zones.hand.retain(|x| *x != object);

    match def.kind {
        CardKind::Creature => {
            state.object_mut(object).summoned = true;
            state.object_mut(object).controller = pid;
            state.player_mut(pid).zones.battlefield.push(object);
        }
        CardKind::Spell => {
            // A carta vai para a pilha, não para o cemitério: ela ainda pode
            // ser respondida e só sai de lá quando resolve.
            state.stack.push(PendingSpell {
                controller: pid,
                object,
                effect: def.effect,
                target,
            });
            // A prioridade sempre volta para o oponente. Quando a pilha já
            // está cheia, `legal_actions` só oferece `Pass` a ele.
            state.priority = state.opponent_of(pid);
            state.phase = Phase::Reaction;
        }
        CardKind::Land => return Err(EngineError::IllegalAction),
    }

    Ok(())
}

fn pass(state: &mut GameState) -> Result<(), EngineError> {
    let pending = state.stack.pop().ok_or(EngineError::IllegalAction)?;
    resolve(state, pending);

    if state.stack.is_empty() {
        state.phase = Phase::Main;
        state.priority = state.active;
    } else {
        state.priority = state.opponent_of(pending.controller);
    }

    Ok(())
}

fn resolve(state: &mut GameState, pending: PendingSpell) {
    // A mágica deixa a pilha para o cemitério do controlador em qualquer
    // desfecho: resolveu ou fizzlou. Ela já foi paga e já foi conjurada.
    state
        .player_mut(pending.controller)
        .zones
        .graveyard
        .push(pending.object);

    if fizzles(state, pending.target) {
        return;
    }

    match pending.effect {
        Effect::None => {}
        Effect::Draw(n) => draw(state, pending.controller, n as usize),
        Effect::DealDamage(n) => match pending.target {
            Some(Target::Player(tp)) => {
                state.player_mut(tp).life -= n as i32;
            }
            Some(Target::Object(tid)) => {
                add_damage(state, tid, n, false);
                cleanup(state);
            }
            None => {}
        },
        Effect::Pump(dp, dt) => {
            if let Some(Target::Object(tid)) = pending.target {
                {
                    let o = state.object_mut(tid);
                    o.buff_power += dp;
                    o.buff_toughness += dt;
                }
                cleanup(state);
            }
        }
    }
}

fn attack(state: &mut GameState, attacker: ObjectId, target: Target) -> Result<(), EngineError> {
    let a = *state.object(attacker);
    let def = card(a.card);
    let power = effective_power(&a);
    let a_deathtouch = def.keywords.contains(&Keyword::Deathtouch);

    match target {
        Target::Player(pid) => {
            state.player_mut(pid).life -= power as i32;
        }
        Target::Object(did) => {
            let d = *state.object(did);
            let ddef = card(d.card);
            let d_power = effective_power(&d);
            let d_deathtouch = ddef.keywords.contains(&Keyword::Deathtouch);

            add_damage(state, did, power, a_deathtouch);

            // Voar: quem não voa não acerta quem voa.
            let a_fly = def.keywords.contains(&Keyword::Flying);
            let d_fly = ddef.keywords.contains(&Keyword::Flying);
            if !a_fly || d_fly {
                add_damage(state, attacker, d_power, d_deathtouch);
            }

            cleanup(state);
        }
    }

    if !def.keywords.contains(&Keyword::Vigilance) {
        state.object_mut(attacker).tapped = true;
    }

    Ok(())
}

fn begin_turn(state: &mut GameState, pid: PlayerId, draw_card: bool) {
    state.turn += 1;
    state.active = pid;
    state.priority = pid;
    state.phase = Phase::Main;
    state.stack.clear();

    let mana = state.lands(pid) as u8;
    state.player_mut(pid).mana = mana;
    state.player_mut(pid).lands_played = 0;

    let battlefield = state.player(pid).zones.battlefield.clone();
    for oid in battlefield {
        state.object_mut(oid).summoned = false;
    }

    if draw_card {
        draw(state, pid, 1);
    }
}

fn draw(state: &mut GameState, pid: PlayerId, n: usize) {
    for _ in 0..n {
        let top = match state.player_mut(pid).zones.library.pop() {
            Some(x) => x,
            None => return,
        };
        state.player_mut(pid).zones.hand.push(top);
    }
}

fn shuffle_library(state: &mut GameState, pid: PlayerId) {
    let mut library = std::mem::take(&mut state.player_mut(pid).zones.library);
    state.rng.shuffle(&mut library);
    state.player_mut(pid).zones.library = library;
}

fn effective_power(o: &Object) -> u8 {
    let p = card(o.card).power as i16 + o.buff_power as i16;
    p.max(0) as u8
}

fn add_damage(state: &mut GameState, id: ObjectId, amount: u8, deathtouch: bool) {
    let o = state.object_mut(id);
    o.damage = o.damage.saturating_add(amount);
    if deathtouch && amount > 0 {
        o.deathtouch_damage = true;
    }
}

/// Move criaturas letais para o cemitério. Ordem deterministicamente
/// estável, porque percorre `Vec` de zonas e não `HashMap`.
fn cleanup(state: &mut GameState) {
    let mut dead: Vec<(PlayerId, ObjectId)> = Vec::new();

    for p in &state.players {
        for &id in &p.zones.battlefield {
            let o = state.object(id);
            let def = card(o.card);
            if def.kind != CardKind::Creature {
                continue;
            }
            let toughness = def.toughness as i16 + o.buff_toughness as i16;
            let lethal = toughness <= 0
                || (o.damage as i16) >= toughness
                || (o.deathtouch_damage && o.damage > 0);
            if lethal {
                dead.push((o.controller, id));
            }
        }
    }

    for (controller, id) in dead {
        state
            .player_mut(controller)
            .zones
            .battlefield
            .retain(|x| *x != id);
        state.player_mut(controller).zones.graveyard.push(id);
        state.object_mut(id).damage = 0;
        state.object_mut(id).deathtouch_damage = false;
        state.object_mut(id).tapped = false;
    }
}

/// Alvo que deixou o campo de batalha antes da resolução.
///
/// Mágica que fizzla não faz nada — mas não é anulada: já foi conjurada, já
/// foi paga, e a carta vai ao cemitério do mesmo jeito.
fn fizzles(state: &GameState, target: Option<Target>) -> bool {
    match target {
        Some(Target::Object(id)) => !is_on_battlefield(state, id),
        _ => false,
    }
}

fn is_on_battlefield(state: &GameState, id: ObjectId) -> bool {
    state
        .players
        .iter()
        .any(|p| p.zones.battlefield.contains(&id))
}

fn needs_target(effect: Effect) -> bool {
    matches!(effect, Effect::DealDamage(_) | Effect::Pump(..))
}

fn candidate_casts(state: &GameState, pid: PlayerId) -> Vec<Action> {
    let mut out = Vec::new();
    if state.stack.len() >= 2 {
        return out;
    }

    let mana = state.player(pid).mana;
    let hand = state.player(pid).zones.hand.clone();

    for oid in hand {
        let def = card(state.object(oid).card);
        if def.kind == CardKind::Land || mana < def.cost {
            continue;
        }
        if needs_target(def.effect) {
            for target in effect_targets(state, def.effect) {
                out.push(Action::Cast {
                    object: oid,
                    target: Some(target),
                });
            }
        } else {
            out.push(Action::Cast {
                object: oid,
                target: None,
            });
        }
    }

    out
}

fn effect_targets(state: &GameState, effect: Effect) -> Vec<Target> {
    let mut out = Vec::new();
    match effect {
        Effect::DealDamage(_) => {
            for p in &state.players {
                out.push(Target::Player(p.id));
            }
            for p in &state.players {
                for &id in &p.zones.battlefield {
                    out.push(Target::Object(id));
                }
            }
        }
        Effect::Pump(..) => {
            for p in &state.players {
                for &id in &p.zones.battlefield {
                    out.push(Target::Object(id));
                }
            }
        }
        _ => {}
    }
    out
}

fn attack_targets(state: &GameState, pid: PlayerId, attacker: ObjectId) -> Vec<Target> {
    let a = state.object(attacker);
    let a_fly = card(a.card).keywords.contains(&Keyword::Flying);

    let mut out = vec![Target::Player(state.opponent_of(pid))];

    for p in &state.players {
        if p.id == pid {
            continue;
        }
        for &id in &p.zones.battlefield {
            let o = state.object(id);
            let d_fly = card(o.card).keywords.contains(&Keyword::Flying);
            if a_fly && !d_fly {
                continue;
            }
            out.push(Target::Object(id));
        }
    }

    out
}
