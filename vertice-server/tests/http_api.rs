//! `POST /matches` — como um cliente entra numa mesa.
//!
//! Sem isso, nenhuma interface consegue jogar: o WebSocket sabe entrar numa
//! partida, mas não sabe criar uma.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use vertice_server::Registry;

async fn criar(app: axum::Router, body: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/matches")
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .expect("requisição bem formada"),
        )
        .await
        .expect("o handler roda");

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("corpo da resposta");
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    (status, json)
}

#[tokio::test]
async fn criar_partida_devolve_id_jogavel() {
    let registry = Arc::new(Registry::new());
    let app = vertice_server::router(registry.clone());

    let (status, body) = criar(app, r#"{"seed":7}"#).await;
    assert_eq!(status, StatusCode::OK);

    let id = body["match_id"].as_str().expect("match_id").to_string();
    assert_eq!(
        body["protocol"].as_u64(),
        Some(u64::from(vertice_server::PROTOCOL_VERSION))
    );

    // O identificador devolvido tem que existir de verdade, com ator vivo:
    // id que não abre conexão é pior que erro, porque só aparece depois.
    assert!(registry.handle(&id).await.is_some());
    assert!(registry.fingerprint(&id).await.is_some());
}

#[tokio::test]
async fn seed_ausente_ainda_cria_partida() {
    let registry = Arc::new(Registry::new());
    let app = vertice_server::router(registry.clone());

    let (status, body) = criar(app, "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["match_id"].is_string());
}

#[tokio::test]
async fn partidas_diferentes_nao_colidem() {
    let registry = Arc::new(Registry::new());
    let app = vertice_server::router(registry.clone());

    let (_, primeiro) = criar(app.clone(), "{}").await;
    let (_, segundo) = criar(app, "{}").await;

    assert_ne!(
        primeiro["match_id"], segundo["match_id"],
        "dois ids iguais seriam duas mesas servindo a mesma partida"
    );
}
