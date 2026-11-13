//! Regressão do registro: as garantias do `registry.toml` são **portão**, não
//! convenção.
//!
//! O bug que esta fatia mata é perder a correspondência entre id e carta. Com
//! id vindo do índice do array, reordenar o arquivo bastava para trocar uma
//! carta por outra e quebrar todo replay existente, em silêncio. Aqui as
//! manobras equivalentes são feitas de propósito e têm que ser recusadas.

use vertice_tools::card::{self, Card};
use vertice_tools::lock::Lock;
use vertice_tools::validate::{check, Snapshot};

const ALFA: &str = r#"
slug = "alfa"
name = "Alfa"
cost = 1
type = "creature"
power = 1
toughness = 1
effect = { kind = "none" }
"#;

const BETA: &str = r#"
slug = "beta"
name = "Beta"
cost = 2
type = "creature"
power = 2
toughness = 2
effect = { kind = "none" }
"#;

const GAMA: &str = r#"
slug = "gama"
name = "Gama"
cost = 1
type = "creature"
power = 1
toughness = 1
effect = { kind = "none" }
"#;

fn registry(next_id: u16, pairs: &[(&str, u16)]) -> String {
    let mut text = format!("next_id = {next_id}\n\n[cards]\n");
    for (slug, id) in pairs {
        text.push_str(&format!("{slug} = {id}\n"));
    }
    text
}

/// Lock equivalente a "estas cartas já foram publicadas com estes ids".
fn lock_for(pairs: &[(u16, &str)]) -> String {
    let mut lock = Lock::default();
    for (id, text) in pairs {
        let parsed: Card = card::parse(text).expect("fixture tem que ser carta válida");
        assert!(
            lock.append(&parsed, *id),
            "fixture não publica o mesmo slug duas vezes"
        );
    }
    lock.to_toml()
}

fn errors(registry_text: &str, lock_text: &str, cards: &[&str]) -> Vec<String> {
    let snapshot =
        Snapshot::from_parts(registry_text, lock_text, cards).expect("fixtures têm que parsear");
    check(&snapshot)
}

fn contains(errors: &[String], needle: &str) -> bool {
    errors.iter().any(|error| error.contains(needle))
}

#[test]
fn catalogo_coerente_passa() {
    let erros = errors(
        &registry(2, &[("alfa", 0), ("beta", 1)]),
        &lock_for(&[(0, ALFA), (1, BETA)]),
        &[ALFA, BETA],
    );
    assert!(erros.is_empty(), "esperava zero erros, veio: {erros:#?}");
}

#[test]
fn inserir_carta_no_meio_e_recusado() {
    // Manobra proibida: enfiar `gama` valendo 1 e empurrar `beta` para 2.
    // É exatamente o que acontecia quando `CardId` era o índice do array —
    // só que ali acontecia calado, e o replay antigo passava a mostrar outra
    // carta no lugar de `beta`.
    let erros = errors(
        &registry(3, &[("alfa", 0), ("gama", 1), ("beta", 2)]),
        &lock_for(&[(0, ALFA), (1, BETA)]),
        &[ALFA, BETA, GAMA],
    );

    assert!(
        contains(&erros, "só anexa no fim"),
        "inserção no meio tinha que ser recusada, veio: {erros:#?}"
    );
    assert!(
        contains(&erros, "mudou de id"),
        "a carta deslocada tinha que ser acusada, veio: {erros:#?}"
    );
}

#[test]
fn anexar_no_fim_e_aceito() {
    // A manobra legal: id novo, maior que todos os publicados.
    let erros = errors(
        &registry(3, &[("alfa", 0), ("beta", 1), ("gama", 2)]),
        &lock_for(&[(0, ALFA), (1, BETA)]),
        &[ALFA, BETA, GAMA],
    );
    assert!(erros.is_empty(), "anexo no fim é legítimo, veio: {erros:#?}");
}

#[test]
fn deletar_carta_da_cauda_e_recusado() {
    // O furo que o `next_id` fecha: apagar a carta do registro E do lock
    // deixava tudo verde, e o id apagado voltava a parecer livre. Como
    // `next_id` vive no registro e não sobe por deleção, a faixa furada
    // denuncia.
    let erros = errors(
        &registry(2, &[("alfa", 0)]), // beta apagada do registro
        "",                           // e a entrada dela no lock também
        &[ALFA],
    );
    assert!(
        contains(&erros, "não tem carta no registro"),
        "deleção da cauda tinha que ser recusada, veio: {erros:#?}"
    );
}

#[test]
fn deletar_carta_do_meio_e_recusado() {
    let erros = errors(
        &registry(3, &[("alfa", 0), ("gama", 2)]),
        "",
        &[ALFA, GAMA],
    );
    assert!(
        contains(&erros, "não tem carta no registro"),
        "deleção do meio tinha que ser recusada, veio: {erros:#?}"
    );
}

#[test]
fn id_acima_do_next_id_e_recusado() {
    // Esquecer de subir `next_id` ao publicar carta nova.
    let erros = errors(&registry(1, &[("alfa", 0), ("beta", 1)]), "", &[ALFA, BETA]);
    assert!(
        contains(&erros, "fora da faixa"),
        "id acima do next_id tinha que ser recusado, veio: {erros:#?}"
    );
}

#[test]
fn lock_truncado_no_meio_e_recusado() {
    // Lock com buraco: a carta do meio saiu do lock mas o resto ficou.
    let mut lock = Lock::default();
    lock.append(&card::parse(ALFA).unwrap(), 0);
    lock.append(&card::parse(GAMA).unwrap(), 2);
    let erros = errors(
        &registry(3, &[("alfa", 0), ("beta", 1), ("gama", 2)]),
        &lock.to_toml(),
        &[ALFA, BETA, GAMA],
    );
    assert!(
        contains(&erros, "não é sequência densa"),
        "lock com buraco tinha que ser recusado, veio: {erros:#?}"
    );
}

#[test]
fn slug_repetido_entre_arquivos_e_recusado() {
    // O registro é indexado por slug, então um slug em dois `set-*` não
    // colide lá: colidia só na emissão, gerando catálogo com id repetido.
    let erros = errors(&registry(1, &[("alfa", 0)]), "", &[ALFA, ALFA]);
    assert!(
        contains(&erros, "aparece em 2 arquivos"),
        "slug repetido tinha que ser recusado, veio: {erros:#?}"
    );
}

#[test]
fn remover_slug_do_registro_e_recusado() {
    let erros = errors(
        &registry(2, &[("alfa", 0)]),
        &lock_for(&[(0, ALFA), (1, BETA)]),
        &[ALFA, BETA],
    );
    assert!(
        contains(&erros, "desapareceu do registro"),
        "remoção de slug tinha que ser recusada, veio: {erros:#?}"
    );
}

#[test]
fn reatribuir_id_de_slug_existente_e_recusado() {
    let erros = errors(
        &registry(4, &[("alfa", 0), ("beta", 3)]),
        &lock_for(&[(0, ALFA), (1, BETA)]),
        &[ALFA, BETA],
    );
    assert!(
        contains(&erros, "mudou de id"),
        "reatribuição de id tinha que ser recusada, veio: {erros:#?}"
    );
}

#[test]
fn reusar_id_para_outro_slug_e_recusado() {
    let erros = errors(
        &registry(2, &[("alfa", 0), ("beta", 0)]),
        &lock_for(&[(0, ALFA), (1, BETA)]),
        &[ALFA, BETA],
    );
    assert!(
        contains(&erros, "duas cartas"),
        "id reutilizado tinha que ser recusado, veio: {erros:#?}"
    );
}

#[test]
fn alterar_carta_publicada_e_recusado() {
    let beta_alterado = BETA.replace("name = \"Beta\"", "name = \"Beta Revisada\"");
    let erros = errors(
        &registry(2, &[("alfa", 0), ("beta", 1)]),
        &lock_for(&[(0, ALFA), (1, BETA)]),
        &[ALFA, &beta_alterado],
    );
    assert!(
        contains(&erros, "carta publicada alterada"),
        "edição de carta publicada tinha que ser recusada, veio: {erros:#?}"
    );
}

#[test]
fn carta_fora_da_curva_exige_justificativa() {
    // 5/5 por 1 mana: muito acima da curva.
    let fora_da_curva = r#"
slug = "colosso"
name = "Colosso"
cost = 1
type = "creature"
power = 5
toughness = 5
effect = { kind = "none" }
"#;
    let erros = errors(&registry(1, &[("colosso", 0)]), "", &[fora_da_curva]);
    assert!(
        contains(&erros, "fora da curva"),
        "custo fora da curva tinha que ser recusado, veio: {erros:#?}"
    );

    let justificada = fora_da_curva.replace(
        "effect = { kind = \"none\" }",
        "effect = { kind = \"none\" }\njustificativa = \"é a carta de assinatura do ato I\"",
    );
    let erros = errors(&registry(1, &[("colosso", 0)]), "", &[&justificada]);
    assert!(
        !contains(&erros, "fora da curva"),
        "com justificativa, a carta passa: {erros:#?}"
    );
}

#[test]
fn palavra_chave_desconhecida_e_recusada() {
    let texto = ALFA.replace(
        "effect = { kind = \"none\" }",
        "keywords = [\"trample\"]\neffect = { kind = \"none\" }",
    );
    let erro = card::parse(&texto).expect_err("trample não existe no Vértice");
    assert!(
        erro.contains("palavra-chave desconhecida"),
        "esperava recusa por palavra-chave, veio: {erro}"
    );
}

#[test]
fn campo_desconhecido_e_recusado() {
    // Erro de digitação em dado de jogo é bug silencioso: tem que ser ruidoso.
    let texto = ALFA.replace("power = 1", "powr = 1");
    let erro = card::parse(&texto).expect_err("campo `powr` não existe");
    assert!(
        erro.contains("powr"),
        "esperava recusa apontando o campo, veio: {erro}"
    );
}

// ------------------------------------------------------------ gatilho de entrada

const COM_GATILHO: &str = r#"
slug = "vigia"
name = "Vigia"
cost = 3
type = "creature"
power = 2
toughness = 2
effect = { kind = "none" }
on_enter = { kind = "draw", amount = 1 }
"#;

#[test]
fn gatilho_em_magica_e_recusado() {
    let magica = r#"
slug = "eco"
name = "Eco"
cost = 2
type = "spell"
effect = { kind = "draw", amount = 2 }
on_enter = { kind = "draw", amount = 1 }
"#;
    let erros = errors(&registry(1, &[("eco", 0)]), "", &[magica]);
    assert!(
        contains(&erros, "mágica com gatilho"),
        "veio: {erros:#?}"
    );
}

#[test]
fn gatilho_com_efeito_de_alvo_e_recusado() {
    // O corte da fatia: gatilho com alvo exigiria escolha no meio da
    // resolução. Recusar no validador é o que impede a carta de existir para
    // depois o motor ignorá-la em silêncio.
    let com_alvo = r#"
slug = "brasa-vigiada"
name = "Brasa Vigiada"
cost = 3
type = "creature"
power = 2
toughness = 2
effect = { kind = "none" }
on_enter = { kind = "deal_damage", amount = 2 }
"#;
    let erros = errors(&registry(1, &[("brasa-vigiada", 0)]), "", &[com_alvo]);
    assert!(contains(&erros, "exige alvo"), "veio: {erros:#?}");
}

#[test]
fn alterar_gatilho_de_carta_publicada_e_recusado() {
    let alterado = COM_GATILHO.replace("amount = 1", "amount = 2");
    let erros = errors(
        &registry(1, &[("vigia", 0)]),
        &lock_for(&[(0, COM_GATILHO)]),
        &[&alterado],
    );
    assert!(
        contains(&erros, "carta publicada alterada"),
        "veio: {erros:#?}"
    );
}

#[test]
fn acrescentar_gatilho_a_carta_publicada_e_recusado() {
    // O caso que o hash condicional precisa cobrir. Se o campo ausente e o
    // presente hasheassem igual, publicar a carta sem gatilho e acrescentá-lo
    // depois passaria batido — e a carta mudaria de comportamento sem que
    // ninguém soubesse.
    let sem_gatilho = COM_GATILHO.replace("on_enter = { kind = \"draw\", amount = 1 }\n", "");
    let erros = errors(
        &registry(1, &[("vigia", 0)]),
        &lock_for(&[(0, &sem_gatilho)]),
        &[COM_GATILHO],
    );
    assert!(
        contains(&erros, "carta publicada alterada"),
        "veio: {erros:#?}"
    );
}

#[test]
fn carta_sem_gatilho_publica_normalmente() {
    // A contraprova: o hash condicional não pode ter quebrado o caminho comum.
    let erros = errors(
        &registry(1, &[("vigia", 0)]),
        &lock_for(&[(0, COM_GATILHO)]),
        &[COM_GATILHO],
    );
    assert!(erros.is_empty(), "carta publicada e não tocada: {erros:#?}");
}
