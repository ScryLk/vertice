//! Binário do servidor.
//!
//! `VERTICE_DATA` diz onde ficam as partidas gravadas (padrão
//! `data/partidas`); `PORT` diz a porta (padrão 8080).
//!
//! Escuta em `127.0.0.1` de propósito: isto é servidor de desenvolvimento.
//! Abrir para fora é decisão de operação, não default de código.

use std::sync::Arc;

use vertice_server::{router, Registry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::env::var("VERTICE_DATA").unwrap_or_else(|_| "data/partidas".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);

    let registry = Arc::new(Registry::open(&data).await?);
    let app = router(registry);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!(
        "Vértice ouvindo em http://{} — partidas gravadas em {data}",
        listener.local_addr()?
    );

    axum::serve(listener, app).await?;
    Ok(())
}
