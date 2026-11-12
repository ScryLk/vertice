//! Ferramentas de conteúdo do Vértice.
//!
//! Duas operações, ambas offline e idempotentes:
//!
//! - `validate` — porta de CI. Confere curva, palavras-chave, ids, o
//!   append-only do registro e se alguma carta publicada mudou. Não escreve.
//! - `generate` — publica. Congela cartas novas no `registry.lock` e
//!   recompila `vertice-core/src/generated/cards.rs`.
//!
//! O TOML é lido aqui e só aqui. O motor continua sem saber o que é arquivo.

pub mod card;
pub mod curve;
pub mod generate;
pub mod lock;
pub mod registry;
pub mod validate;

use std::path::{Path, PathBuf};

/// Raiz do workspace.
///
/// `CARGO_MANIFEST_DIR` é o diretório deste crate, resolvido em tempo de
/// compilação. Fixar assim evita depender do diretório de quem roda o
/// comando — e um caminho relativo ao cwd quebraria em CI.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("vertice-tools precisa estar dentro do workspace")
        .to_path_buf()
}
