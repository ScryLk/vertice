//! O bot jogando de verdade, pelo mesmo caminho que um cliente usa.
//!
//! O bot é decidido dentro da task da partida, então o cliente não manda nada
//! por ele: manda a própria ação e recebe uma view por ação — inclusive as do
//! bot, que chegam como push. Este teste é o que prova que essa engrenagem
//! gira no servidor, e não só na biblioteca.

mod common;

use std::sync::Arc;

use common::{servir, Client};
use vertice_bot::Level;
use vertice_server::protocol::ActionDto;
use vertice_server::Registry;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn humano_que_so_passa_a_vez_perde_para_o_bot() {
    let registry = Arc::new(Registry::new());
    let addr = servir(registry.clone()).await;
    let id = registry
        .create_vs_bot(Some(7), Level::Formando)
        .await
        .expect("cria a partida contra o bot");

    let mut eu = Client::connect(addr, &id, 0).await;

    let mut jogadas_do_bot = 0i64;

    for _ in 0..400 {
        if eu.view.winner.is_some() {
            break;
        }

        let seq_antes = eu.seq;
        eu.act(ActionDto::EndTurn).await;

        // Uma view por ação: a minha primeiro, depois as do bot, até a
        // prioridade voltar para mim ou a partida acabar.
        loop {
            eu.next().await;
            if eu.view.winner.is_some() || eu.view.priority == eu.view.you.id {
                break;
            }
        }

        jogadas_do_bot += eu.seq as i64 - seq_antes as i64 - 1;
    }

    assert_eq!(
        eu.view.winner,
        Some(1),
        "quem não faz nada tem que perder para o bot"
    );
    assert!(
        jogadas_do_bot > 20,
        "o bot tem que ter jogado de verdade, jogou {jogadas_do_bot} vezes"
    );
    assert!(
        eu.view.legal_actions.is_empty(),
        "partida encerrada não oferece ação"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn o_bot_responde_sem_o_humano_pedir() {
    let registry = Arc::new(Registry::new());
    let addr = servir(registry.clone()).await;
    let id = registry
        .create_vs_bot(Some(11), Level::Formando)
        .await
        .expect("cria a partida contra o bot");

    let mut eu = Client::connect(addr, &id, 0).await;
    assert_eq!(eu.view.you.id, 0, "o humano joga no assento 0");
    assert!(eu.view.winner.is_none());

    let seq_antes = eu.seq;
    eu.act(ActionDto::EndTurn).await;

    // Só o humano mandou uma ação. Se o `seq` andar mais que isso, foi o bot
    // que jogou — ninguém pediu nada por ele.
    loop {
        eu.next().await;
        if eu.view.winner.is_some() || eu.view.priority == eu.view.you.id {
            break;
        }
    }

    assert!(
        eu.seq > seq_antes + 1,
        "o bot tinha que ter jogado sozinho: seq foi de {seq_antes} para {}",
        eu.seq
    );
}
