use std::collections::BTreeMap;

use crate::action::Action;
use crate::cards::Effect;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::rng::Rng;

/// Zonas. Todas são vetores de `ObjectId`.
///
/// `BTreeMap` no estado e `Vec` nas zonas são escolha de determinismo, não
/// de performance. `HashMap` tem ordem de iteração instável e quebraria
/// replay de forma difícil de diagnosticar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Zones {
    pub library: Vec<ObjectId>,
    pub hand: Vec<ObjectId>,
    pub battlefield: Vec<ObjectId>,
    pub graveyard: Vec<ObjectId>,
    /// Zona de comando. Reservada; o M0 não usa.
    pub command: Vec<ObjectId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub id: PlayerId,
    pub name: &'static str,
    pub life: i32,
    pub zones: Zones,
    /// Mana disponível neste turno. No M0 é derivada do número de terrenos
    /// no início do turno; terrenos não viram. Ver nota em `engine`.
    pub mana: u8,
    pub lands_played: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Object {
    pub id: ObjectId,
    pub card: CardId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub tapped: bool,
    /// Entrou em jogo neste turno: não pode atacar.
    pub summoned: bool,
    pub damage: u8,
    pub buff_power: i8,
    pub buff_toughness: i8,
    /// Recebeu dano de fonte com toque mortal.
    pub deathtouch_damage: bool,
}

impl Object {
    pub fn new(id: ObjectId, card: CardId, owner: PlayerId) -> Self {
        Self {
            id,
            card,
            owner,
            controller: owner,
            tapped: false,
            summoned: false,
            damage: 0,
            buff_power: 0,
            buff_toughness: 0,
            deathtouch_damage: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Jogador com prioridade age livremente.
    Main,
    /// Uma mágica está na pilha e o oponente tem uma janela de resposta.
    /// Só existe **uma** janela por mágica: pilha máxima de 2.
    Reaction,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Player(PlayerId),
    Object(ObjectId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PendingSpell {
    pub controller: PlayerId,
    pub object: ObjectId,
    pub effect: Effect,
    pub target: Option<Target>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    pub players: Vec<Player>,
    pub objects: BTreeMap<ObjectId, Object>,
    pub next_object: u32,
    pub active: PlayerId,
    pub priority: PlayerId,
    pub turn: u32,
    pub phase: Phase,
    pub stack: Vec<PendingSpell>,
    pub rng: Rng,
    pub log: Vec<Action>,
}

impl GameState {
    pub fn player(&self, id: PlayerId) -> &Player {
        &self.players[id as usize]
    }

    pub fn player_mut(&mut self, id: PlayerId) -> &mut Player {
        &mut self.players[id as usize]
    }

    pub fn object(&self, id: ObjectId) -> &Object {
        self.objects.get(&id).expect("ObjectId inexistente no estado")
    }

    pub fn object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.objects
            .get_mut(&id)
            .expect("ObjectId inexistente no estado")
    }

    pub fn opponent_of(&self, id: PlayerId) -> PlayerId {
        debug_assert_eq!(self.players.len(), 2, "M0 é estritamente 1v1");
        if id == 0 {
            1
        } else {
            0
        }
    }

    /// Terrenos no campo de batalha — a fonte de mana do M0.
    pub fn lands(&self, id: PlayerId) -> usize {
        self.player(id)
            .zones
            .battlefield
            .iter()
            .filter(|oid| crate::cards::card(self.object(**oid).card).kind == crate::cards::CardKind::Land)
            .count()
    }

    /// Soma de debug do estado. Barato de comparar e não depende de `Debug`
    /// de estruturas profundas — usado na reconciliação cliente/servidor.
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        fnv(&mut h, self.turn as u64);
        fnv(&mut h, self.active as u64);
        fnv(&mut h, self.priority as u64);
        fnv(&mut h, self.rng.state());
        fnv(&mut h, self.stack.len() as u64);
        for p in &self.stack {
            fnv(&mut h, p.object.0 as u64);
            fnv(&mut h, p.controller as u64);
        }
        for p in &self.players {
            fnv(&mut h, p.life as u64);
            fnv(&mut h, p.mana as u64);
            fnv(&mut h, p.zones.library.len() as u64);
            fnv(&mut h, p.zones.hand.len() as u64);
            fnv(&mut h, p.zones.battlefield.len() as u64);
            fnv(&mut h, p.zones.graveyard.len() as u64);
        }
        for o in self.objects.values() {
            fnv(&mut h, o.id.0 as u64);
            fnv(&mut h, o.card as u64);
            fnv(&mut h, o.damage as u64);
            fnv(&mut h, o.buff_power as i64 as u64);
            fnv(&mut h, o.buff_toughness as i64 as u64);
            fnv(&mut h, o.tapped as u64);
        }
        h
    }
}

/// FNV-1a. Usado só para fingerprint, nunca para decisão de regra.
fn fnv(h: &mut u64, v: u64) {
    *h ^= v;
    *h = h.wrapping_mul(0x0000_0100_0000_01B3);
}
