//! A curva de custo é uma decisão, e decisão sem teste regride em silêncio.
//!
//! O que está fixado aqui é a **calibragem**, não a fórmula: a base é a média
//! do corpo e não o menor dos dois, e o peso da palavra-chave é por palavra. As
//! duas coisas foram medidas contra um corpus externo de criaturas baunilha, e
//! as duas já estiveram erradas neste projeto — a primeira porque foi escolhida
//! para acomodar uma carta só, a segunda porque tratava palavras diferentes
//! como se custassem o mesmo.
//!
//! O último teste é o que fecha o portão: nenhuma carta publicada pode sair da
//! banda por causa de mudança na curva.

use vertice_tools::card::{Card, Effect, Keyword, Kind};
use vertice_tools::curve::{self, BAND};
use vertice_tools::validate::{self, Snapshot};
use vertice_tools::workspace_root;

fn criatura(power: i8, toughness: i8, keywords: &[Keyword]) -> Card {
    Card {
        slug: "teste".to_string(),
        name: "Teste".to_string(),
        cost: 0,
        kind: Kind::Creature,
        power,
        toughness,
        keywords: keywords.to_vec(),
        effect: Effect::None,
        justificativa: None,
    }
}

fn magica(effect: Effect) -> Card {
    Card {
        slug: "teste".to_string(),
        name: "Teste".to_string(),
        cost: 0,
        kind: Kind::Spell,
        power: 0,
        toughness: 0,
        keywords: Vec::new(),
        effect,
        justificativa: None,
    }
}

#[test]
fn corpo_simetrico_custa_o_proprio_corpo() {
    for n in 0..=6 {
        assert_eq!(
            curve::reference(&criatura(n, n, &[])),
            Some(i32::from(n)),
            "criatura {n}/{n}"
        );
    }
}

#[test]
fn corpo_assimetrico_paga_a_media_para_cima() {
    // A regressão que importa. `min(P,T)` cobrava 1 por 2/1 e por 1/2; o
    // corpus diz que as duas custam o mesmo que 2/2 — o corpo é o total, não
    // a metade que interessa.
    assert_eq!(curve::reference(&criatura(2, 1, &[])), Some(2));
    assert_eq!(curve::reference(&criatura(1, 2, &[])), Some(2));
    assert_eq!(curve::reference(&criatura(2, 2, &[])), Some(2));

    // Ímpar arredonda para cima: 5/2 é 3, não 2.
    assert_eq!(curve::reference(&criatura(3, 2, &[])), Some(3));
    assert_eq!(curve::reference(&criatura(2, 3, &[])), Some(3));

    assert_eq!(curve::reference(&criatura(0, 1, &[])), Some(1));
    assert_eq!(curve::reference(&criatura(0, 0, &[])), Some(0));
}

#[test]
fn palavra_chave_tem_peso_proprio() {
    // Antes era +1 para qualquer palavra. Medido: voar cobra ~1, as outras
    // duas cobram ~0 — o corpo que as carrega já pagou parte delas.
    assert_eq!(curve::reference(&criatura(2, 2, &[Keyword::Flying])), Some(3));
    assert_eq!(
        curve::reference(&criatura(2, 2, &[Keyword::Vigilance])),
        Some(2)
    );
    assert_eq!(
        curve::reference(&criatura(2, 2, &[Keyword::Deathtouch])),
        Some(2)
    );
}

#[test]
fn peso_de_palavra_chave_soma() {
    assert_eq!(
        curve::reference(&criatura(2, 2, &[Keyword::Flying, Keyword::Deathtouch])),
        Some(3)
    );
    assert_eq!(
        curve::reference(&criatura(
            2,
            2,
            &[Keyword::Vigilance, Keyword::Deathtouch]
        )),
        Some(2)
    );
}

#[test]
fn magica_mantem_a_regua_anterior() {
    assert_eq!(curve::reference(&magica(Effect::DealDamage(3))), Some(2));
    assert_eq!(curve::reference(&magica(Effect::DealDamage(1))), Some(1));
    assert_eq!(curve::reference(&magica(Effect::Draw(2))), Some(2));
    // Pump segue a régua do dano: +2/+2 é o mesmo que 2 de dano, que custa 1.
    assert_eq!(curve::reference(&magica(Effect::Pump(2, 2))), Some(1));
    assert_eq!(curve::reference(&magica(Effect::Pump(-3, -3))), Some(2));
    assert_eq!(curve::reference(&magica(Effect::None)), None);
}

#[test]
fn nenhuma_carta_publicada_sai_da_banda() {
    // O portão de verdade. Trocar a curva é barato; trocar a curva e descobrir
    // três dias depois que uma carta publicada virou ilegal não é. Isto roda o
    // validador real sobre `cards/`, não uma cópia da regra.
    let snapshot = Snapshot::from_disk(&workspace_root())
        .unwrap_or_else(|erros| panic!("cards/ tem que parsear: {erros:#?}"));

    let fora: Vec<String> = validate::check(&snapshot)
        .into_iter()
        .filter(|erro| erro.contains("fora da curva"))
        .collect();

    assert!(
        fora.is_empty(),
        "a curva deixou cartas publicadas fora da banda (delta > {BAND}):\n{}",
        fora.join("\n")
    );
}
