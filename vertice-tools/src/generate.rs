//! Compila o dado em código Rust.
//!
//! `generate` é o passo de **publicação**: acrescenta ao `registry.lock` as
//! cartas que ainda não estavam lá e reescreve o catálogo gerado. Depois
//! disso a carta está publicada, e a entrada do lock congela o conteúdo dela.
//!
//! A saída tem que ser byte a byte estável: nada de data, caminho absoluto ou
//! ordem herdada do sistema de arquivos. Gerar duas vezes tem que dar o mesmo
//! arquivo — é o que torna o diff legível e o portão honesto.

use std::path::{Path, PathBuf};

use crate::card::{Card, Effect, Kind};
use crate::validate::Snapshot;

pub fn generated_path(root: &Path) -> PathBuf {
    root.join("vertice-core/src/generated/cards.rs")
}

pub fn emit(cards: &[(u16, &Card)]) -> String {
    let uses_keyword = cards.iter().any(|(_, card)| !card.keywords.is_empty());

    let mut out = String::new();
    out.push_str("//! GERADO POR `vertice-tools` — NÃO EDITE À MÃO.\n");
    out.push_str("//!\n");
    out.push_str("//! Fonte: `cards/registry.toml` e `cards/set-*/*.toml`.\n");
    out.push_str("//! Regenerar: `cargo run -p vertice-tools -- generate`\n");
    out.push_str("//!\n");
    out.push_str("//! O `id` de cada carta vem do registro e nunca muda. A ordem desta lista\n");
    out.push_str("//! é por id, e **ordem não é identidade**: quem identifica a carta é o campo\n");
    out.push_str("//! `id`. Reordenar este arquivo não quebra replay; mudar um id, sim.\n");
    out.push('\n');

    out.push_str("use crate::cards::{CardDef, CardKind, Effect");
    if uses_keyword {
        out.push_str(", Keyword");
    }
    out.push_str("};\n\n");

    out.push_str(&format!(
        "/// Catálogo do conjunto. {} cartas.\n",
        cards.len()
    ));
    out.push_str("pub static CATALOG: &[CardDef] = &[\n");

    for (id, card) in cards {
        out.push_str("    CardDef {\n");
        out.push_str(&format!("        id: {id},\n"));
        out.push_str(&format!("        name: {},\n", rust_string(&card.name)));
        out.push_str(&format!("        cost: {},\n", card.cost));
        out.push_str(&format!("        kind: {},\n", kind_expr(card.kind)));
        out.push_str(&format!("        power: {},\n", card.power));
        out.push_str(&format!("        toughness: {},\n", card.toughness));
        out.push_str(&format!("        keywords: {},\n", keywords_expr(card)));
        out.push_str(&format!("        effect: {},\n", effect_expr(card.effect)));
        out.push_str(&format!("        on_enter: {},\n", on_enter_expr(card)));
        out.push_str(&format!("        on_die: {},\n", on_die_expr(card)));
        out.push_str("    },\n");
    }

    out.push_str("];\n");
    out
}

/// Publica: valida, congela as cartas novas no lock e regenera o catálogo.
pub fn run(root: &Path) -> Result<String, Vec<String>> {
    let snapshot = Snapshot::from_disk(root)?;
    let mut errors = crate::validate::check(&snapshot);
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let ordered = crate::validate::ordered(&snapshot)?;

    let mut lock = snapshot.lock.clone();
    let mut published = 0usize;
    for (id, card) in &ordered {
        if lock.append(card, *id) {
            published += 1;
        }
    }

    let lock_path = root.join("cards/registry.lock");
    write(&lock_path, &lock.to_toml())?;

    let path = generated_path(root);
    write(&path, &emit(&ordered))?;

    Ok(format!(
        "{published} carta(s) nova(s) publicada(s); {} cartas no catálogo",
        ordered.len()
    ))
}

fn kind_expr(kind: Kind) -> &'static str {
    match kind {
        Kind::Land => "CardKind::Land",
        Kind::Creature => "CardKind::Creature",
        Kind::Spell => "CardKind::Spell",
    }
}

fn keywords_expr(card: &Card) -> String {
    if card.keywords.is_empty() {
        return "&[]".to_string();
    }
    // Ordenadas: a ordem no arquivo é do autor, a do código é canônica.
    let mut keywords = card.keywords.clone();
    keywords.sort_unstable();
    let items: Vec<String> = keywords
        .iter()
        .map(|k| format!("Keyword::{}", k.rust_name()))
        .collect();
    format!("&[{}]", items.join(", "))
}

fn on_enter_expr(card: &Card) -> String {
    trigger_expr(card.on_enter)
}

fn on_die_expr(card: &Card) -> String {
    trigger_expr(card.on_die)
}

fn trigger_expr(trigger: Option<Effect>) -> String {
    match trigger {
        None => "None".to_string(),
        Some(effect) => format!("Some({})", effect_expr(effect)),
    }
}

fn effect_expr(effect: Effect) -> String {
    match effect {
        Effect::None => "Effect::None".to_string(),
        Effect::DealDamage(amount) => format!("Effect::DealDamage({amount})"),
        Effect::Draw(amount) => format!("Effect::Draw({amount})"),
        Effect::Pump(power, toughness) => format!("Effect::Pump({power}, {toughness})"),
        Effect::GainLife(amount) => format!("Effect::GainLife({amount})"),
        Effect::LoseLife(amount) => format!("Effect::LoseLife({amount})"),
    }
}

fn rust_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn write(path: &Path, contents: &str) -> Result<(), Vec<String>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| vec![format!("{}: {e}", parent.display())])?;
    }
    // Escreve ao lado e renomeia: quem estiver lendo nunca vê arquivo pela metade.
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, contents)
        .map_err(|e| vec![format!("{}: {e}", temporary.display())])?;
    std::fs::rename(&temporary, path).map_err(|e| vec![format!("{}: {e}", path.display())])
}
