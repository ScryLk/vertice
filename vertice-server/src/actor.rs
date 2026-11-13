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
use vertice_bot::{choose_action, Level};
use vertice_core::rng::Rng;
use vertice_core::{
    apply, legal_actions, new_game, view, CardId, EngineError, GameState, MatchRecord, PlayerId,
};

use crate::protocol::{ActionDto, PlayerViewDto, RejectReason, ServerMessage};
use crate::store::Store;

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

/// Quantas jogadas seguidas o bot pode fazer antes de o ator devolver o
/// controle. Um bot contra bot joga muitas; o limite existe para que um bug
/// no bot não trave a task para sempre.
const LIMITE_DE_JOGADAS_DO_BOT: usize = 512;

pub struct Actor {
    id: String,
    state: GameState,
    record: MatchRecord,
    seq: u64,
    next_generation: u64,
    seats: [Seat; 2],
    store: Option<Store>,
    /// Assento controlado pelo bot, se houver.
    bots: [Option<Level>; 2],
    /// Semente do bot derivada da partida: a mesma partida dá as mesmas
    /// decisões. Não entra no registro porque o registro guarda as *ações*,
    /// que é o que o replay precisa.
    bot_rng: Rng,
}

impl Actor {
    pub fn new(
        id: String,
        deck0: &[CardId],
        deck1: &[CardId],
        seed: u64,
        store: Option<Store>,
        bots: [Option<Level>; 2],
    ) -> Result<Self, EngineError> {
        Ok(Actor {
            state: new_game(deck0, deck1, seed)?,
            record: MatchRecord::new(deck0.to_vec(), deck1.to_vec(), seed),
            id,
            store,
            seq: 0,
            next_generation: 1,
            seats: [Seat::empty(), Seat::empty()],
            bots,
            bot_rng: Rng::new(seed ^ 0xb07_b07_b07),
        })
    }

    /// Retoma uma partida gravada: rejoga o registro e assume o estado.
    ///
    /// É a mesma operação que o replay de teste faz. A diferença é que aqui o
    /// resultado vira um ator vivo, que volta a aceitar ação — e o `seq` volta
    /// ao número de ações já aplicadas, para o cliente não precisar saber que
    /// o servidor reiniciou.
    pub fn restore(
        id: String,
        record: MatchRecord,
        store: Option<Store>,
    ) -> Result<Self, EngineError> {
        let state = record.replay()?;
        let seq = record.actions.len() as u64;
        let seed = record.seed;
        Ok(Actor {
            state,
            record,
            id,
            store,
            seq,
            next_generation: 1,
            seats: [Seat::empty(), Seat::empty()],
            // Partida contra bot não é gravada (ver `Registry::create_vs_bot`),
            // então o que volta do disco é sempre humano contra humano.
            bots: [None; 2],
            bot_rng: Rng::new(seed ^ 0xb07_b07_b07),
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
                    // Se o bot abre a partida, ele joga agora — antes de o
                    // humano receber a view, para ela já vir com a jogada dele.
                    self.pump_bots().await;

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
        // Partida encerrada vem antes de tudo: nem índice nem legalidade
        // importam, e a razão da recusa tem que ser essa.
        if self.state.winner.is_some() {
            return self.reject(seat, RejectReason::GameOver).await;
        }

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
                self.commit(action).await;
                self.pump_bots().await;
            }
            Err(EngineError::GameOver) => self.reject(seat, RejectReason::GameOver).await,
            Err(_) => self.reject(seat, RejectReason::IllegalAction).await,
        }
    }

    /// Grava a ação já aplicada e avisa os dois assentos.
    async fn commit(&mut self, action: vertice_core::Action) {
        self.seq += 1;
        self.record.push(action);

        // Persistir **antes** de avisar. O cliente só fica sabendo de uma ação
        // depois que ela está durável, então reiniciar o servidor logo em
        // seguida não perde nada que alguém já viu.
        self.persist().await;

        for viewer in 0..2u8 {
            let message = ServerMessage::View {
                seq: self.seq,
                view: self.view_for(viewer),
            };
            self.push(viewer, message).await;
        }
    }

    /// Deixa o bot jogar enquanto for a vez dele.
    ///
    /// Roda dentro da task da partida, então a decisão do bot nunca corre em
    /// paralelo com uma ação de cliente: a mensagem seguinte só é lida depois
    /// que o bot terminou de jogar. O limite de jogadas seguidas existe para
    /// que um bug no bot não trave a task para sempre.
    async fn pump_bots(&mut self) {
        for _ in 0..LIMITE_DE_JOGADAS_DO_BOT {
            if self.state.winner.is_some() {
                return;
            }

            let seat = self.state.priority;
            let Some(level) = self.bots[seat as usize] else {
                return;
            };
            let Some(action) = choose_action(&self.state, seat, level, &mut self.bot_rng) else {
                return;
            };
            if apply(&mut self.state, action).is_err() {
                return;
            }

            self.commit(action).await;
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

    async fn persist(&self) {
        let Some(store) = &self.store else {
            return;
        };

        if let Err(error) = store.save(&self.id, &self.record).await {
            // Falhar aqui não derruba a partida: ela segue viva em memória, e
            // recusar a ação já aplicada seria mentir sobre o estado. Mas o
            // fato de não estar durável precisa ser visível em algum lugar, e
            // até existir log estruturado esse lugar é o stderr.
            eprintln!(
                "[vertice] falha ao persistir a partida {}: {error}",
                self.id
            );
        }
    }
}
