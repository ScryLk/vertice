//! `Action` na forma de fio.
//!
//! A conversão é um `match` exaustivo de propósito. Quando alguém adicionar
//! uma variante em `Action`, o compilador quebra aqui e obriga a decidir o
//! formato dela na rede. É o único mecanismo anti-deriva que existe para um
//! protocolo escrito à mão, e ele só funciona enquanto ninguém trocar o
//! `match` por um `_ =>`.

use serde::{Deserialize, Serialize};
use vertice_core::{Action, ObjectId, PlayerId, Target};

/// Ação trafegável.
///
/// `object` e `attacker` são `ObjectId`, não `CardId`: o que circula é a
/// *cópia* dentro da partida, nunca a carta do catálogo. Sem isso, duas
/// cópias da mesma carta seriam indistinguíveis na rede.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionDto {
    PlayLand {
        object: u32,
    },
    Cast {
        object: u32,
        target: Option<TargetDto>,
    },
    Attack {
        attacker: u32,
        target: TargetDto,
    },
    Pass,
    EndTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TargetDto {
    Player { player: PlayerId },
    Object { object: u32 },
}

impl From<Target> for TargetDto {
    fn from(target: Target) -> Self {
        match target {
            Target::Player(player) => TargetDto::Player { player },
            Target::Object(object) => TargetDto::Object { object: object.0 },
        }
    }
}

impl From<TargetDto> for Target {
    fn from(target: TargetDto) -> Self {
        match target {
            TargetDto::Player { player } => Target::Player(player),
            TargetDto::Object { object } => Target::Object(ObjectId(object)),
        }
    }
}

impl From<Action> for ActionDto {
    fn from(action: Action) -> Self {
        match action {
            Action::PlayLand { object } => ActionDto::PlayLand { object: object.0 },
            Action::Cast { object, target } => ActionDto::Cast {
                object: object.0,
                target: target.map(TargetDto::from),
            },
            Action::Attack { attacker, target } => ActionDto::Attack {
                attacker: attacker.0,
                target: target.into(),
            },
            Action::Pass => ActionDto::Pass,
            Action::EndTurn => ActionDto::EndTurn,
        }
    }
}

impl From<ActionDto> for Action {
    fn from(action: ActionDto) -> Self {
        match action {
            ActionDto::PlayLand { object } => Action::PlayLand {
                object: ObjectId(object),
            },
            ActionDto::Cast { object, target } => Action::Cast {
                object: ObjectId(object),
                target: target.map(Target::from),
            },
            ActionDto::Attack { attacker, target } => Action::Attack {
                attacker: ObjectId(attacker),
                target: target.into(),
            },
            ActionDto::Pass => Action::Pass,
            ActionDto::EndTurn => Action::EndTurn,
        }
    }
}
