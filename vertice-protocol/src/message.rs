//! O envelope: toda mensagem tem `type`, e toda mensagem de jogo tem `seq`.
//!
//! Duas coisas que parecem detalhe e não são:
//!
//! - **`protocol` no `Join`.** O cliente declara a versão que fala. Versão
//!   diferente é recusa imediata, não tentativa de compatibilidade. Um
//!   protocolo de jogo não precisa negociar; precisa falhar alto.
//! - **`seq`.** É o índice da mensagem na conversa. Sem ele, resposta fora de
//!   ordem depois de reconexão é indistinguível de resposta certa, e o cliente
//!   prevê em cima de estado que o servidor já abandonou. Com ele, o cliente
//!   descarta o que é velho e pede reidratação quando o buraco é grande.

use serde::{Deserialize, Serialize};
use vertice_core::PlayerId;

use super::{ActionDto, PlayerViewDto};

/// Versão do protocolo.
///
/// Sobe a cada mudança de forma. A 2 acrescentou `winner` na view: cliente da
/// 1 não sabe que a partida acabou, então falhar na porta é melhor que ficar
/// mandando ação para um jogo encerrado.
pub const PROTOCOL_VERSION: u32 = 2;

/// Do cliente para o servidor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Primeira mensagem da conexão.
    ///
    /// A partida vem na URL (`/ws/{match_id}`), não aqui: um identificador em
    /// dois lugares é um identificador que uma hora discorda de si mesmo.
    Join { protocol: u32, seat: PlayerId },
    /// Uma ação, com o índice que o cliente espera ver confirmado.
    Action { seq: u64, action: ActionDto },
    /// "Perdi o fio": o cliente pede a view de novo, sem tentar adivinhar.
    Resync { seq: u64 },
}

/// Do servidor para o cliente.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Resposta ao `Join`: a partir daqui o cliente tem estado.
    Welcome {
        protocol: u32,
        seat: PlayerId,
        seq: u64,
        view: PlayerViewDto,
    },
    /// Nova view depois de uma ação aceita ou de um `Resync`.
    View { seq: u64, view: PlayerViewDto },
    /// A ação foi recusada e o estado **não** mudou.
    Rejected { seq: u64, reason: RejectReason },
    /// Erro de conexão, não de jogo.
    Error { kind: ErrorKind },
}

/// Por que uma ação foi recusada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    /// Fora de `legal_actions`.
    IllegalAction,
    /// A partida já terminou.
    GameOver,
    /// O `seq` está atrás do que o servidor já aplicou.
    Stale,
}

/// Por que a conexão não pode continuar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    ProtocolMismatch,
    UnknownMatch,
    SeatTaken,
}
