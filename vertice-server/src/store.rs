//! Onde uma partida dorme.
//!
//! Um arquivo JSON por partida, reescrito inteiro a cada ação aceita. É
//! deliberadamente o mais burro que funciona: sem banco, sem migração, sem
//! processo para operar. `cat` no arquivo mostra a partida inteira, e apagar
//! o diretório reseta o servidor.
//!
//! **O custo é conhecido:** reescrever o registro inteiro a cada ação é O(n)
//! por ação. Com partidas de algumas centenas de ações e um registro de
//! dezenas de KB, isso não é problema. Quando for — e vai ser, quando o pool
//! passar de 200 cartas e as partidas de 1000 ações — a troca é para log
//! append-only, sem mudar quem chama: `save` continua querendo o registro.
//!
//! Gravação é atômica: escreve ao lado e renomeia. Quem lê nunca vê arquivo
//! pela metade, nem se o processo morrer no meio.

use std::path::{Path, PathBuf};

use vertice_core::MatchRecord;

use crate::protocol::MatchRecordDto;

#[derive(Clone, Debug)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn open(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Store { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self, match_id: &str) -> PathBuf {
        self.dir.join(format!("{match_id}.json"))
    }

    pub async fn save(&self, match_id: &str, record: &MatchRecord) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(&MatchRecordDto::from(record))
            .map_err(std::io::Error::other)?;

        let path = self.path(match_id);
        let temporary = path.with_extension("json.tmp");
        tokio::fs::write(&temporary, text).await?;
        tokio::fs::rename(&temporary, &path).await
    }

    /// Todas as partidas gravadas, em ordem de nome — determinístico, porque
    /// ordem de `read_dir` não é.
    pub async fn load_all(&self) -> std::io::Result<Vec<(String, MatchRecord)>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                files.push(path);
            }
        }
        files.sort();

        let mut loaded = Vec::with_capacity(files.len());
        for path in files {
            let Some(match_id) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            let text = tokio::fs::read_to_string(&path).await?;
            let dto: MatchRecordDto =
                serde_json::from_str(&text).map_err(std::io::Error::other)?;
            loaded.push((match_id, MatchRecord::from(dto)));
        }
        Ok(loaded)
    }
}
