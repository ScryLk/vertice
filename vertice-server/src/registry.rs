//! Registro de partidas vivas.
//!
//! É o mínimo para o servidor existir: um `match_id` e um canal para a task
//! que é dona daquela partida. Sem fila, sem matchmaking, sem persistência —
//! cada uma dessas coisas entra quando tiver alguém do outro lado pedindo.
//!
//! A partida vive **em memória**. Reiniciar o processo perde as partidas em
//! andamento. Persistir é o próximo passo, e o `MatchRecordDto` já está
//! pronto para isso: é só quem grava e quem lê.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::{mpsc, oneshot, RwLock};
use vertice_core::{CardId, EngineError};

use crate::actor::{Actor, Command};

pub struct Registry {
    matches: RwLock<BTreeMap<String, mpsc::Sender<Command>>>,
    counter: AtomicU64,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            matches: RwLock::new(BTreeMap::new()),
            counter: AtomicU64::new(1),
        }
    }

    /// Cria a partida e devolve o identificador.
    pub async fn create(
        &self,
        deck0: Vec<CardId>,
        deck1: Vec<CardId>,
        seed: u64,
    ) -> Result<String, EngineError> {
        let actor = Actor::new(&deck0, &deck1, seed)?;
        let id = format!("partida-{}", self.counter.fetch_add(1, Ordering::Relaxed));
        let (sender, commands) = mpsc::channel(64);
        actor.spawn(commands);
        self.matches.write().await.insert(id.clone(), sender);
        Ok(id)
    }

    pub async fn handle(&self, match_id: &str) -> Option<mpsc::Sender<Command>> {
        self.matches.read().await.get(match_id).cloned()
    }

    /// Fingerprint do estado da partida. Diagnóstico e teste; não vai para a
    /// rede, porque não é informação que o cliente deva ter.
    pub async fn fingerprint(&self, match_id: &str) -> Option<u64> {
        let sender = self.handle(match_id).await?;
        let (reply, answer) = oneshot::channel();
        sender.send(Command::Fingerprint { reply }).await.ok()?;
        answer.await.ok()
    }
}
