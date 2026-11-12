//! Registro de partidas.
//!
//! É o mínimo para o servidor existir: um `match_id` e um canal para a task
//! que é dona daquela partida. Sem fila e sem matchmaking — cada um entra
//! quando tiver alguém do outro lado pedindo.
//!
//! Com `Store`, a partida sobrevive a reinício: `open` relê o que está
//! gravado, rejoga cada registro e devolve um ator vivo. É a mesma operação
//! do replay de teste, só que o resultado aceita ação em vez de virar
//! asserção.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::{mpsc, oneshot, RwLock};
use vertice_core::{CardId, EngineError};

use crate::actor::{Actor, Command};
use crate::store::Store;

pub struct Registry {
    matches: RwLock<BTreeMap<String, mpsc::Sender<Command>>>,
    counter: AtomicU64,
    store: Option<Store>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Registro sem persistência: partida morre junto com o processo.
    pub fn new() -> Self {
        Self::build(None)
    }

    /// Abre o diretório de partidas e retoma tudo que estiver gravado.
    ///
    /// Um arquivo que não rejoga é erro de inicialização, não aviso: partida
    /// ilegível significa que o motor mudou de comportamento debaixo de um
    /// registro antigo, e subir o servidor fingindo que está tudo bem seria
    /// esconder exatamente o que o replay existe para denunciar.
    pub async fn open(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let store = Store::open(dir)?;
        let registry = Self::build(Some(store.clone()));

        let mut next = 1u64;
        for (id, record) in store.load_all().await? {
            if let Some(number) = id
                .strip_prefix("partida-")
                .and_then(|rest| rest.parse::<u64>().ok())
            {
                next = next.max(number.saturating_add(1));
            }

            let actor = Actor::restore(id.clone(), record, Some(store.clone())).map_err(|error| {
                std::io::Error::other(format!("a partida `{id}` não rejoga: {error:?}"))
            })?;
            let (sender, commands) = mpsc::channel(64);
            actor.spawn(commands);
            registry.matches.write().await.insert(id, sender);
        }

        // Novo id nunca colide com o que já está no disco.
        registry.counter.store(next, Ordering::Relaxed);
        Ok(registry)
    }

    fn build(store: Option<Store>) -> Self {
        Registry {
            matches: RwLock::new(BTreeMap::new()),
            counter: AtomicU64::new(1),
            store,
        }
    }

    /// Cria a partida e devolve o identificador.
    pub async fn create(
        &self,
        deck0: Vec<CardId>,
        deck1: Vec<CardId>,
        seed: u64,
    ) -> Result<String, EngineError> {
        let id = format!("partida-{}", self.counter.fetch_add(1, Ordering::Relaxed));
        let actor = Actor::new(id.clone(), &deck0, &deck1, seed, self.store.clone())?;

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
