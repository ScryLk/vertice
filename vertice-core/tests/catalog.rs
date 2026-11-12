//! Invariantes do catálogo gerado.
//!
//! Não testa conteúdo — testa que o artefato commitado em
//! `src/generated/cards.rs` respeita o contrato do registro. Se alguém
//! regenerar com uma ferramenta quebrada, quebra aqui e não num replay de
//! seis meses atrás.

use vertice_core::cards::{card, CATALOG, CATALOG_LEN};

#[test]
fn ids_sao_densos_e_crescentes() {
    for (index, def) in CATALOG.iter().enumerate() {
        assert_eq!(
            def.id as usize, index,
            "catálogo fora de ordem por id: a posição {index} carrega o id {}",
            def.id
        );
    }
    assert_eq!(CATALOG.len(), CATALOG_LEN);
}

#[test]
fn card_por_id_devolve_a_mesma_carta() {
    for def in CATALOG {
        assert_eq!(card(def.id).name, def.name);
        assert_eq!(card(def.id).id, def.id);
    }
}

#[test]
fn ids_de_carta_sao_unicos() {
    let mut ids: Vec<_> = CATALOG.iter().map(|def| def.id).collect();
    ids.sort_unstable();
    let antes = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), antes, "id repetido no catálogo gerado");
}

#[test]
#[should_panic(expected = "fora do catálogo")]
fn id_inexistente_e_panico() {
    let _ = card(CATALOG_LEN as u16);
}
