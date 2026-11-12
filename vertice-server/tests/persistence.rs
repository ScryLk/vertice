//! A partida tem que sobreviver ao processo.
//!
//! Reconexão já estava provada com o servidor vivo. Aqui o servidor **morre**:
//! o registro antigo vai embora, e um registro novo abre o mesmo diretório,
//! rejoga o que está gravado e volta a aceitar ação no mesmo estado.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use common::{baralho, jogar, servir, Client};
use vertice_server::Registry;

fn diretorio(nome: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vertice-{nome}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partida_sobrevive_a_reinicio_do_servidor() {
    let dir = diretorio("persistencia");

    // ------------------------------------------------ servidor 1
    let primeiro = Arc::new(Registry::open(&dir).await.expect("abre o diretório"));
    let addr = servir(primeiro.clone()).await;
    let id = primeiro
        .create(baralho(0), baralho(3), 2024)
        .await
        .expect("cria a partida");

    let mut a = Client::connect(addr, &id, 0).await;
    let mut b = Client::connect(addr, &id, 1).await;

    let andados = jogar(&mut a, &mut b, 12).await;
    assert!(andados >= 8, "a partida precisa andar; andou {andados}");

    let fingerprint = primeiro.fingerprint(&id).await.expect("fingerprint");
    let view = a.view.clone();
    let seq = a.seq;

    // O processo morre. Aqui só some o registro e as conexões — os atores do
    // primeiro registro terminam sozinhos quando o canal fecha. O que importa
    // é que o registro novo não os conhece.
    drop(a);
    drop(b);
    drop(primeiro);

    // ------------------------------------------------ servidor 2
    let segundo = Arc::new(Registry::open(&dir).await.expect("relê o diretório"));
    let addr = servir(segundo.clone()).await;

    let mut a = Client::connect(addr, &id, 0).await;
    assert_eq!(a.seq, seq, "depois do reinício, o mesmo índice");
    assert_eq!(a.view, view, "e a mesma view");
    assert_eq!(
        segundo.fingerprint(&id).await.expect("fingerprint"),
        fingerprint,
        "o estado tem que ser rejogado idêntico"
    );

    // E volta a aceitar ação.
    let mut b = Client::connect(addr, &id, 1).await;
    let depois = jogar(&mut a, &mut b, 8).await;
    assert!(depois > 0, "a partida retomada tem que continuar jogável");
    assert!(a.seq > seq, "e o índice tem que avançar");

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registro_ilegivel_impede_o_servidor_de_subir() {
    let dir = diretorio("registro-ruim");
    std::fs::create_dir_all(&dir).expect("cria o diretório");
    // Baralho de uma carta: não é partida válida, então não rejoga.
    std::fs::write(
        dir.join("partida-1.json"),
        r#"{"engine_version":"0.1.0","card_set_version":"m0.1","seed":1,"deck0":[0],"deck1":[0],"hand0":[],"hand1":[],"actions":[]}"#,
    )
    .expect("grava um registro ruim de propósito");

    let erro = match Registry::open(&dir).await {
        Ok(_) => panic!("registro que não rejoga tem que impedir a subida"),
        Err(erro) => erro,
    };
    assert!(
        erro.to_string().contains("não rejoga"),
        "a mensagem tem que dizer o que houve, veio: {erro}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
