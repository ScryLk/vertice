//! Tudo que atravessa a rede.
//!
//! A regra que organiza este módulo: **o motor nunca vira bytes sozinho.**
//! `vertice-core` não tem `serde` e não vai ter. O que existe aqui é a
//! tradução explícita entre os tipos do motor e a forma que viaja.
//!
//! Escrita à mão de propósito. Um `#[derive(Serialize)]` no core seria menos
//! código e cobraria o preço que o projeto não paga: a primeira dependência
//! dentro do motor. O custo da mão é duplicação; o pagamento é que a conversão
//! é um `match` exaustivo, e o compilador avisa quando o motor ganha uma
//! variante que a rede ainda não sabe falar.

mod action;
mod message;
mod record;
mod view;

pub use action::{ActionDto, TargetDto};
pub use message::{ClientMessage, ErrorKind, RejectReason, ServerMessage, PROTOCOL_VERSION};
pub use record::MatchRecordDto;
pub use view::{
    CardViewDto, KeywordDto, OpponentViewDto, PhaseDto, PlayerViewDto, SelfViewDto, StackViewDto,
};
