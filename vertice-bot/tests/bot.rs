//! O bot só vale se for legal, determinístico e melhor que o acaso.
//!
//! As três coisas são testáveis e são as três que importam. "Parece decente"
//! não é critério; ganhar de um adversário aleatório é.
//!
//! ## Por que todo placar é espelhado
//!
//! O assento 0 sai na frente, e neste motor isso pesa muito: espelhando
//! Formando contra Formando, o assento 0 venceu 14 de 16. Medir força com a
//! política forte sempre no assento 0 mediria, no fundo, vantagem de saída.
//! Cada confronto aqui roda nas duas orientações, então o viés de assento
//! cancela e sobra a diferença entre as políticas.

use vertice_bot::{choose_action, Level};
use vertice_core::cards::CATALOG_LEN;
use vertice_core::rng::Rng;
use vertice_core::*;

fn baralho(offset: u16) -> Vec<CardId> {
    let n = CATALOG_LEN as u16;
    (0..100u16)
        .map(|i| ((i * 7 + offset) % n) as CardId)
        .collect()
}

/// Quem joga de um lado da mesa. O aleatório existe só como régua: se a
/// heurística não bate o acaso com folga, ela não vale nada.
#[derive(Clone, Copy, Debug)]
enum Politica {
    Nivel(Level),
    Aleatorio,
}

fn escolher(state: &GameState, pid: PlayerId, politica: Politica, rng: &mut Rng) -> Option<Action> {
    let legais = legal_actions(state, pid);
    if legais.is_empty() {
        return None;
    }

    match politica {
        Politica::Nivel(nivel) => choose_action(state, pid, nivel, rng),
        Politica::Aleatorio => Some(legais[rng.range(legais.len())]),
    }
}

struct Disputa {
    state: GameState,
    decisoes: u32,
}

/// Uma partida inteira. `apply` só recebe o que a política escolheu — se o bot
/// devolvesse ação ilegal, isto entra em pânico aqui.
fn disputa(seed: u64, p0: Politica, p1: Politica, caos: u64) -> Disputa {
    // **O mesmo baralho dos dois lados.** Com baralhos diferentes, medir
    // força vira loteria: 100 cartas em 13 valores deixa 9 valores com 8
    // cópias e 4 com 7, e quais são depende do deslocamento — então
    // `baralho(0)` e `baralho(3)` não têm a mesma composição. Trocar as
    // políticas de assento trocava também o baralho, e o placar media a
    // diferença entre os decks.
    let deck = baralho(0);
    let mut state = new_game(&deck, &deck, seed).expect("partida válida");
    let mut rngs = [Rng::new(caos), Rng::new(caos ^ 0x9e37_79b9_7f4a_7c15)];

    let mut decisoes = 0;
    for _ in 0..4000 {
        let pid = state.priority;
        let politica = if pid == 0 { p0 } else { p1 };

        let Some(acao) = escolher(&state, pid, politica, &mut rngs[pid as usize]) else {
            break;
        };
        apply(&mut state, acao).expect("o bot só pode escolher ação legal");
        decisoes += 1;

        if state.winner.is_some() {
            break;
        }
    }

    Disputa { state, decisoes }
}

/// (vitórias de `p`, vitórias de `q`), com cada seed jogada duas vezes e as
/// políticas trocando de assento. É o que cancela a vantagem de sair primeiro.
fn placar(p: Politica, q: Politica, rodadas: u64) -> (u32, u32) {
    let (mut vitorias_p, mut vitorias_q) = (0, 0);

    for seed in 0..rodadas {
        match disputa(seed, p, q, seed * 31 + 7).state.winner {
            Some(0) => vitorias_p += 1,
            Some(1) => vitorias_q += 1,
            Some(outro) => panic!("vencedor impossível: {outro}"),
            None => panic!("a partida não terminou (seed {seed}, p no assento 0)"),
        }

        // A **mesma** seed: o mesmo embaralhamento nas duas orientações. Com
        // faixas de seed diferentes, o viés de assento não cancela — ele varia
        // com o embaralhamento (medido: 10x2 numa faixa, 4x8 noutra).
        match disputa(seed, q, p, seed * 17 + 3).state.winner {
            Some(0) => vitorias_q += 1,
            Some(1) => vitorias_p += 1,
            Some(outro) => panic!("vencedor impossível: {outro}"),
            None => panic!("a partida não terminou (seed {seed}, p no assento 1)"),
        }
    }

    (vitorias_p, vitorias_q)
}

#[test]
fn so_escolhe_acao_legal() {
    let mut state = new_game(&baralho(0), &baralho(3), 11).expect("partida válida");
    let mut rng = Rng::new(5);

    for _ in 0..400 {
        let pid = state.priority;
        let legais = legal_actions(&state, pid);
        let Some(acao) = choose_action(&state, pid, Level::Calouro, &mut rng) else {
            break;
        };

        assert!(
            legais.contains(&acao),
            "o bot devolveu `{acao:?}`, que não está em legal_actions"
        );

        apply(&mut state, acao).expect("ação legal");
        if state.winner.is_some() {
            break;
        }
    }
}

#[test]
fn mesma_entrada_mesma_decisao() {
    // Vale para os dois níveis, e o do calouro é o que importa: ele consome
    // RNG, então a semente tem que produzir sempre o mesmo caminho.
    for nivel in [Level::Formando, Level::Calouro] {
        let a = disputa(7, Politica::Nivel(nivel), Politica::Nivel(Level::Calouro), 42);
        let b = disputa(7, Politica::Nivel(nivel), Politica::Nivel(Level::Calouro), 42);

        assert_eq!(
            a.state.fingerprint(),
            b.state.fingerprint(),
            "duas partidas com a mesma semente divergiram em {nivel:?}"
        );
        assert_eq!(a.decisoes, b.decisoes);
    }
}

#[test]
fn sem_acao_legal_devolve_none() {
    let state = new_game(&baralho(0), &baralho(3), 3).expect("partida válida");
    let sem_prioridade = state.opponent_of(state.priority);
    let mut rng = Rng::new(1);

    assert_eq!(
        choose_action(&state, sem_prioridade, Level::Formando, &mut rng),
        None,
        "quem não tem a prioridade não tem ação legal"
    );
}

#[test]
fn partida_de_bot_termina() {
    let resultado = disputa(
        2024,
        Politica::Nivel(Level::Formando),
        Politica::Nivel(Level::Formando),
        0,
    );

    assert!(
        resultado.state.winner.is_some(),
        "partida de bot que não termina em 4000 decisões é laço"
    );
    assert!(resultado.decisoes > 20);
}

#[test]
fn a_balanca_esta_calibrada() {
    // Controle do arranjo. Com a mesma política dos dois lados e a mesma
    // seed nas duas orientações, as duas partidas de cada rodada são a
    // *mesma* partida — então o placar tem que ser exatamente N para cada
    // lado. Se não for, `placar` está trocando assento ou seed, e todo
    // número deste arquivo está contaminado.
    let rodadas = 12;
    let (a, b) = placar(
        Politica::Nivel(Level::Formando),
        Politica::Nivel(Level::Formando),
        rodadas,
    );
    println!("controle: {a} x {b}");

    assert_eq!(
        (a, b),
        (rodadas as u32, rodadas as u32),
        "o arranjo espelhado não é neutro: {a} x {b}"
    );
}

#[test]
fn formando_atropela_o_aleatorio() {
    let (formando, aleatorio) = placar(Politica::Nivel(Level::Formando), Politica::Aleatorio, 12);
    println!("formando {formando} x {aleatorio} aleatório");

    assert!(
        formando >= 20,
        "o sério tem que atropelar o acaso, veio {formando} x {aleatorio} de 24"
    );
}

#[test]
fn formando_vence_calouro() {
    // O teste que dá sentido ao nível de dificuldade: a diferença entre eles
    // é só a frequência de erro, então ela tem que aparecer no placar.
    let (formando, calouro) = placar(
        Politica::Nivel(Level::Formando),
        Politica::Nivel(Level::Calouro),
        12,
    );
    println!("formando {formando} x {calouro} calouro");

    assert!(
        formando >= 16,
        "o nível sério tem que bater o nível que erra com folga: {formando} x {calouro} de 24"
    );
}

#[test]
fn calouro_ainda_bate_o_aleatorio() {
    // A escada tem que ser monótona: Formando > Calouro > Aleatório.
    //
    // **A barra é acoplada ao baralho, não só ao bot.** `provisional_deck`
    // cicla o catálogo inteiro, então mexer no pool troca a composição dos
    // decks e o placar junto — e o corpo do conjunto mudou muito: 13, 50, 66,
    // 80, 92, 102 cartas, cada salto com cartas de remoção que abrem espaço
    // para erro caro. Medido com 102: 17 x 7. A barra em 16 é margem clara
    // sobre o acaso (2 de cada 3), e o que este teste protege é a escada
    // continuar de pé, não um número exato. Re-meça quando o pool mudar; não
    // baixe sem medir.
    let (calouro, aleatorio) = placar(Politica::Nivel(Level::Calouro), Politica::Aleatorio, 12);
    println!("calouro {calouro} x {aleatorio} aleatório");

    assert!(
        calouro >= 16,
        "o calouro tem que ser melhor que o acaso: {calouro} x {aleatorio} de 24"
    );
}

