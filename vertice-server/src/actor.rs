//! Uma partida é uma task, e ela é dona do estado.
//!
//! Nada de `Mutex` em cima do `GameState`: quem quer mudar o estado manda uma
//! mensagem. A task processa uma de cada vez, então "duas ações ao mesmo
//! tempo" não existe por construção — que é exatamente a garantia que o
//! servidor autoritativo precisa.
//!
//! **Reconexão é a mesma operação que entrar.** O `GameState` mora aqui, não
//! na conexão, então quando um cliente cai a partida continua onde estava. Ao
//! voltar, ele recebe `Welcome` com a view atual e o mesmo `seq`.
//!
//! Duas sutilezas que não são detalhe:
//!
//! - **Geração.** Reconectar substitui o canal de saída do assento. O `Detach`
//!   da conexão velha pode chegar *depois* do `Attach` da nova; sem o número
//!   de geração, ele apagaria o canal novo e o jogador ficaria mudo.
//! - **Cada assento recebe a própria view.** A redação é do motor; o ator só
//!   chama `view(state, seat)` uma vez por assento. Nunca há uma view
//!   "compartilhada" que possa vazar mão alheia.

use tokio::sync::{mpsc, oneshot};
use vertice_core::{
    apply, legal_actions, new_game, view, CardId, EngineError, GameState, PlayerId,
};

use crate::protocol::{ActionDto, PlayerViewDto, RejectReason, ServerMessage};

pub enum Command {
    /// Um cliente entrou ou voltou. Substitui o canal de saída do assento.
    Attach {
        seat: PlayerId,
        outbound: mpsc::Sender<ServerMessage>,
        reply: oneshot::Sender<Attached>,
    },
    /// A conexão `generation` caiu. Só apaga o canal se ainda for a atual.
    Detach { seat: PlayerId, generation: u64 },
    Act {
        seat: PlayerId,
        seq: u64,
        action: ActionDto,
    },
    /// "Perdi o fio": empurra a view atual de novo.
    Snapshot { seat: PlayerId },
    /// Só para teste e diagnóstico: o estado do motor por dentro.
    Fingerprint { reply: oneshot::Sender<u64> },
}

pub struct Attached {
    pub generation: u64,
    pub seq: u64,
    pub view: PlayerViewDto,
}

struct Seat {
    generation: u64,
    outbound: Option<mpsc::Sender<ServerMessage>>,
}

impl Seat {
    fn empty() -> Self {
        Seat {
            generation: 0,
            outbound: None,
        }
    }
}

pub struct Actor {
    state: GameState,
    seq: u64,
    next_generation: u64,
    seats: [Seat; 2],
}

impl Actor {
    pub fn new(deck0: &[CardId], deck1: &[CardId], seed: u64) -> Result<Self, EngineError> {
        Ok(Actor {
            state: new_game(deck0, deck1, seed)?,
            seq: 0,
            next_generation: 1,
            seats: [Seat::empty(), Seat::empty()],
        })
    }

    pub fn spawn(self, commands: mpsc::Receiver<Command>) {
        tokio::spawn(self.run(commands));
    }

    async fn run(mut self, mut commands: mpsc::Receiver<Command>) {
        while let Some(command) = commands.recv().await {
            match command {
                Command::Attach {
                    seat,
                    outbound,
                    reply,
                } => {
                    let generation = self.next_generation;
                    self.next_generation += 1;
                    self.seats[seat as usize] = Seat {
                        generation,
                        outbound: Some(outbound),
                    };
                    let _ = reply.send(Attached {
                        generation,
                        seq: self.seq,
                        view: self.view_for(seat),
                    });
                }
                Command::Detach { seat, generation } => {
                    let slot = &mut self.seats[seat as usize];
                    if slot.generation == generation {
                        slot.outbound = None;
                    }
                }
                Command::Act { seat, seq, action } => self.act(seat, seq, action).await,
                Command::Snapshot { seat } => {
                    let message = ServerMessage::View {
                        seq: self.seq,
                        view: self.view_for(seat),
                    };
                    self.push(seat, message).await;
                }
                Command::Fingerprint { reply } => {
                    let _ = reply.send(self.state.fingerprint());
                }
            }
        }
    }

    async fn act(&mut self, seat: PlayerId, seq: u64, action: ActionDto) {
        // `seq` é o índice que o cliente acredita ser o atual. Divergiu, ele
        // está prevendo em cima de estado que o servidor passou; o certo é
        // pedir reidratação, não aplicar por cima.
        if seq != self.seq {
            return self.reject(seat, RejectReason::Stale).await;
        }

        let action: vertice_core::Action = action.into();
        if !legal_actions(&self.state, seat).contains(&action) {
            return self.reject(seat, RejectReason::IllegalAction).await;
        }

        match apply(&mut self.state, action) {
            Ok(()) => {
                self.seq += 1;
                for viewer in 0..2u8 {
                    let message = ServerMessage::View {
                        seq: self.seq,
                        view: self.view_for(viewer),
                    };
                    self.push(viewer, message).await;
                }
            }
            Err(EngineError::GameOver) => self.reject(seat, RejectReason::GameOver).await,
            Err(_) => self.reject(seat, RejectReason::IllegalAction).await,
        }
    }

    async fn reject(&mut self, seat: PlayerId, reason: RejectReason) {
        let message = ServerMessage::Rejected {
            seq: self.seq,
            reason,
        };
        self.push(seat, message).await;
    }

    async fn push(&mut self, seat: PlayerId, message: ServerMessage) {
        let Some(outbound) = &self.seats[seat as usize].outbound else {
            return;
        };
        // Canal fechado = cliente sumiu no meio do envio. O estado da partida
        // não depende de ninguém estar ouvindo.
        let _ = outbound.send(message).await;
    }

    fn view_for(&self, seat: PlayerId) -> PlayerViewDto {
        PlayerViewDto::from(&view(&self.state, seat))
    }
}
