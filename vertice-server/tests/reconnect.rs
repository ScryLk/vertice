//! O marco da M1.3, automatizado.
//!
//! Dois clientes WebSocket headless jogam a mesma partida. Um deles tem a
//! conexão derrubada no meio. Ele reconecta e a partida continua exatamente
//! de onde estava — mesmo `seq`, mesma `view`, mesmo `fingerprint`.
//!
//! "Abri dois navegadores e funcionou" não é prova, e este arquivo existe
//! para nunca precisar dessa frase.

mod common;

use std::sync::Arc;

use common::{baralho, jogar, passo, receive, send, servir, Client};
use vertice_server::protocol::*;
use vertice_server::Registry;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partida_sobrevive_a_queda_de_um_dos_clientes() {
    let registry = Arc::new(Registry::new());
    let addr = servir(registry.clone()).await;
    let id = registry
        .create(baralho(0), baralho(3), 2024)
        .await
        .expect("cria a partida");

    let mut a = Client::connect(addr, &id, 0).await;
    let mut b = Client::connect(addr, &id, 1).await;

    let andados = jogar(&mut a, &mut b, 15).await;
    assert!(
        andados >= 10,
        "a partida precisa andar de verdade antes da queda; andou {andados} passos"
    );
    assert!(
        a.view.winner.is_none(),
        "a queda tem que ser testada com a partida viva, não encerrada"
    );

    let fingerprint_antes = registry.fingerprint(&id).await.expect("fingerprint");
    let view_antes = a.view.clone();
    let seq_antes = a.seq;
    assert_eq!(
        a.seq, b.seq,
        "os dois assentos têm que concordar sobre o índice"
    );

    // A queda.
    drop(a);

    // A volta: o mesmo assento, uma conexão nova.
    let mut a = Client::connect(addr, &id, 0).await;

    assert_eq!(a.seq, seq_antes, "reconexão tem que voltar no mesmo índice");
    assert_eq!(
        a.view, view_antes,
        "a view depois de reconectar tem que ser exatamente a de antes"
    );
    assert_eq!(
        registry.fingerprint(&id).await.expect("fingerprint"),
        fingerprint_antes,
        "o estado do motor não pode ter se mexido enquanto ninguém estava olhando"
    );

    // E a partida continua de onde parou.
    let depois = jogar(&mut a, &mut b, 15).await;
    assert!(
        depois > 0,
        "a partida tem que continuar depois da reconexão; andou {depois} passos"
    );
    assert!(a.seq > seq_antes, "o índice tem que avançar");
    assert_eq!(a.seq, b.seq, "os dois assentos continuam de acordo");
    assert_eq!(
        a.view.winner, b.view.winner,
        "se a partida acabou nesse meio-tempo, os dois têm que saber igual"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partida_joga_ate_o_fim_e_os_dois_assentos_sabem_quem_venceu() {
    let registry = Arc::new(Registry::new());
    let addr = servir(registry.clone()).await;
    let id = registry
        .create(baralho(0), baralho(3), 2024)
        .await
        .expect("cria a partida");

    let mut a = Client::connect(addr, &id, 0).await;
    let mut b = Client::connect(addr, &id, 1).await;

    let mut passos = 0;
    while a.view.winner.is_none() && passos < 400 {
        let acted = if a.view.priority == 0 {
            passo(&mut a, &mut b).await
        } else {
            passo(&mut b, &mut a).await
        };
        if !acted {
            break;
        }
        passos += 1;
    }

    let vencedor = a.view.winner.expect("a partida tem que terminar");
    assert_eq!(
        b.view.winner,
        Some(vencedor),
        "os dois assentos veem o mesmo resultado"
    );
    assert!(
        a.view.legal_actions.is_empty(),
        "partida encerrada não oferece ação"
    );
    assert!(b.view.legal_actions.is_empty());

    // E o servidor recusa com a razão certa, não com "ação ilegal".
    let seq = a.seq;
    a.send(&ClientMessage::Action {
        seq,
        action: ActionDto::EndTurn,
    })
    .await;
    match a.next().await {
        ServerMessage::Rejected { reason, .. } => assert_eq!(reason, RejectReason::GameOver),
        other => panic!("esperava `Rejected(GameOver)`, veio {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn o_servidor_recusa_acao_ilegal_sem_mexer_no_estado() {
    let registry = Arc::new(Registry::new());
    let addr = servir(registry.clone()).await;
    let id = registry
        .create(baralho(0), baralho(3), 77)
        .await
        .expect("cria a partida");

    let mut a = Client::connect(addr, &id, 0).await;
    let antes = registry.fingerprint(&id).await.expect("fingerprint");

    // Objeto que não existe: fora de `legal_actions`.
    let seq = a.seq;
    a.send(&ClientMessage::Action {
        seq,
        action: ActionDto::PlayLand { object: 9999 },
    })
    .await;
    match a.next().await {
        ServerMessage::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::IllegalAction);
        }
        other => panic!("esperava `Rejected`, veio {other:?}"),
    }
    assert_eq!(
        registry.fingerprint(&id).await.expect("fingerprint"),
        antes,
        "ação recusada não pode mexer no estado"
    );

    // Índice que o servidor não reconhece: o cliente está prevendo em cima de
    // um passado que não existe mais.
    let seq = a.seq + 50;
    a.send(&ClientMessage::Action {
        seq,
        action: ActionDto::EndTurn,
    })
    .await;
    match a.next().await {
        ServerMessage::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::Stale);
        }
        other => panic!("esperava `Rejected`, veio {other:?}"),
    }
    assert_eq!(
        registry.fingerprint(&id).await.expect("fingerprint"),
        antes,
        "índice atrasado também não pode mexer no estado"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partida_inexistente_e_recusada() {
    let registry = Arc::new(Registry::new());
    let addr = servir(registry).await;

    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws/nao-existe"))
        .await
        .expect("abre o WebSocket");
    send(
        &mut socket,
        &ClientMessage::Join {
            protocol: PROTOCOL_VERSION,
            seat: 0,
        },
    )
    .await;

    match receive(&mut socket).await {
        ServerMessage::Error { kind } => assert_eq!(kind, ErrorKind::UnknownMatch),
        other => panic!("esperava `Error`, veio {other:?}"),
    }
}
