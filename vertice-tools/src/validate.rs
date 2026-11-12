//! O validador. É o produto desta fatia: o catálogo ser dado só vale se o
//! dado não puder ser quebrado por acidente.
//!
//! `check` é puro — recebe `Snapshot`, devolve erros. Nada de disco aqui,
//! porque é isso que permite o teste de regressão do registro rodar em
//! memória, sobre um registro inventado, sem precisar de repositório.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::card::{self, Card};
use crate::lock::Lock;
use crate::registry::Registry;

pub struct Snapshot {
    pub registry: Registry,
    pub lock: Lock,
    pub cards: Vec<Card>,
}

impl Snapshot {
    /// Monta um snapshot a partir de texto, sem tocar em disco. Usado pelos
    /// testes; qualquer divergência com `from_disk` aparece lá.
    pub fn from_parts(
        registry_text: &str,
        lock_text: &str,
        card_texts: &[&str],
    ) -> Result<Self, Vec<String>> {
        let registry = Registry::parse(registry_text).map_err(|e| vec![e])?;
        let lock = Lock::parse(lock_text).map_err(|e| vec![e])?;

        let mut errors = Vec::new();
        let mut cards = Vec::new();
        for text in card_texts {
            match card::parse(text) {
                Ok(parsed) => cards.push(parsed),
                Err(e) => errors.push(e),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Snapshot {
            registry,
            lock,
            cards,
        })
    }

    pub fn from_disk(root: &Path) -> Result<Self, Vec<String>> {
        let cards_dir = root.join("cards");
        let mut errors = Vec::new();

        let registry_path = cards_dir.join("registry.toml");
        let registry = match read(&registry_path) {
            Ok(text) => match Registry::parse(&text) {
                Ok(registry) => registry,
                Err(e) => return Err(vec![format!("{}: {e}", registry_path.display())]),
            },
            Err(e) => return Err(vec![format!("{e}")]),
        };

        let lock_path = cards_dir.join("registry.lock");
        let lock = if lock_path.exists() {
            match read(&lock_path) {
                Ok(text) => match Lock::parse(&text) {
                    Ok(lock) => lock,
                    Err(e) => return Err(vec![format!("{}: {e}", lock_path.display())]),
                },
                Err(e) => return Err(vec![format!("{e}")]),
            }
        } else {
            Lock::default()
        };

        let mut cards = Vec::new();
        for set_dir in set_dirs(&cards_dir, &mut errors) {
            for path in card_files(&set_dir, &mut errors) {
                let text = match read(&path) {
                    Ok(text) => text,
                    Err(e) => {
                        errors.push(e);
                        continue;
                    }
                };
                match card::parse(&text) {
                    Ok(parsed) => {
                        let stem = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        if stem != parsed.slug {
                            errors.push(format!(
                                "{}: nome do arquivo não bate com o slug `{}`",
                                path.display(),
                                parsed.slug
                            ));
                        }
                        cards.push(parsed);
                    }
                    Err(e) => errors.push(format!("{}: {e}", path.display())),
                }
            }
        }

        // Carta solta fora de `set-*` seria carta que ninguém valida: fica
        // invisível para o gerador e para o portão. Ruidoso é melhor.
        match std::fs::read_dir(&cards_dir) {
            Ok(entries) => {
                for entry in entries {
                    let Ok(entry) = entry else { continue };
                    let path = entry.path();
                    let is_toml = path.extension().and_then(|e| e.to_str()) == Some("toml");
                    let is_registry =
                        path.file_name().and_then(|n| n.to_str()) == Some("registry.toml");
                    if is_toml && !is_registry {
                        errors.push(format!(
                            "{}: carta fora de um diretório `set-*` não é lida — mova para cards/set-m1/",
                            path.display()
                        ));
                    }
                }
            }
            Err(e) => errors.push(format!("{}: {e}", cards_dir.display())),
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Snapshot {
            registry,
            lock,
            cards,
        })
    }
}

/// Cartas ordenadas por `CardId`, que é a ordem do catálogo gerado.
pub fn ordered(snapshot: &Snapshot) -> Result<Vec<(u16, &Card)>, Vec<String>> {
    let mut ordered = Vec::with_capacity(snapshot.cards.len());
    let mut errors = Vec::new();
    for card in &snapshot.cards {
        match snapshot.registry.id_of(&card.slug) {
            Some(id) => ordered.push((id, card)),
            None => errors.push(format!(
                "`{}` não está em cards/registry.toml: toda carta precisa de id no registro",
                card.slug
            )),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    ordered.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.slug.cmp(&b.1.slug)));

    // Guarda final antes de emitir: o catálogo gerado não pode ter id
    // repetido, porque `card()` faz busca binária pela chave e escolheria
    // entre os duplicados sem contrato nenhum.
    for pair in ordered.windows(2) {
        if pair[0].0 == pair[1].0 {
            errors.push(format!(
                "duas cartas disputam o id {}: `{}` e `{}`",
                pair[0].0, pair[0].1.slug, pair[1].1.slug
            ));
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }
    Ok(ordered)
}

pub fn check(snapshot: &Snapshot) -> Vec<String> {
    let mut errors = Vec::new();

    for card in &snapshot.cards {
        errors.extend(card::validate(card));
    }

    // 0. Um slug, um arquivo. O registro é indexado por slug, então um slug
    //    repetido em dois `set-*` não colide lá — colide só na hora de emitir,
    //    e aí o catálogo sai com dois `CardDef` do mesmo id e a busca binária
    //    de `card()` escolhe um dos dois sem contrato.
    let mut file_count: BTreeMap<&str, usize> = BTreeMap::new();
    for card in &snapshot.cards {
        *file_count.entry(card.slug.as_str()).or_default() += 1;
    }
    for (slug, count) in &file_count {
        if *count > 1 {
            errors.push(format!(
                "slug `{slug}` aparece em {count} arquivos: um slug, um arquivo"
            ));
        }
    }

    // 1. Um id, uma carta.
    let mut id_to_slug: BTreeMap<u16, &str> = BTreeMap::new();
    for (slug, &id) in &snapshot.registry.by_slug {
        if let Some(other) = id_to_slug.insert(id, slug.as_str()) {
            errors.push(format!(
                "id {id} usado por duas cartas: `{other}` e `{slug}` — id não se reutiliza"
            ));
        }
    }

    // 2. A faixa `0..next_id` tem que estar inteira preenchida.
    //
    //    É esta igualdade que transforma "apagar carta" em erro. Sem ela, quem
    //    apagasse a carta do registro, do arquivo e do lock veria tudo verde —
    //    e o id apagado voltaria a parecer livre. `next_id` não está no lock
    //    de propósito: é a prova que não some junto com a carta.
    let expected: BTreeSet<u16> = (0..snapshot.registry.next_id).collect();
    let actual: BTreeSet<u16> = snapshot.registry.by_slug.values().copied().collect();
    for missing in expected.difference(&actual) {
        errors.push(format!(
            "id {missing} não tem carta no registro (next_id = {}): carta apagada? Registro só cresce.",
            snapshot.registry.next_id
        ));
    }
    for extra in actual.difference(&expected) {
        errors.push(format!(
            "id {extra} está fora da faixa: incremente `next_id` em cards/registry.toml (hoje {})",
            snapshot.registry.next_id
        ));
    }

    // 3. O lock é um prefixo denso de `0..` — nenhuma entrada removida.
    let lock_ids: BTreeSet<u16> = snapshot.lock.entries.iter().map(|e| e.id).collect();
    let published = snapshot.lock.entries.len();
    if lock_ids.len() != published {
        errors.push("cards/registry.lock tem id repetido".to_string());
    } else if lock_ids != (0..published as u16).collect() {
        errors.push(
            "cards/registry.lock não é sequência densa a partir do 0: entrada apagada ou id \
             fora de ordem. O lock é append-only."
                .to_string(),
        );
    }

    // 2. Registro e arquivos falam a mesma língua.
    let with_file: BTreeSet<&str> = snapshot.cards.iter().map(|c| c.slug.as_str()).collect();
    for slug in snapshot.registry.by_slug.keys() {
        if !with_file.contains(slug.as_str()) {
            errors.push(format!(
                "registro aponta `{slug}`, mas não existe arquivo em cards/set-*"
            ));
        }
    }

    // 3. Append-only: carta nova só entra no fim da numeração.
    //
    // É esta regra que recusa "inserir no meio". Colar id 5 entre o 4 e o 6
    // deslocaria todos os ids seguintes e quebraria todo replay existente —
    // em silêncio, que é o pior tipo de quebra.
    let mut new_cards: Vec<(u16, &str)> = snapshot
        .registry
        .by_slug
        .iter()
        .filter(|(slug, _)| snapshot.lock.get(slug.as_str()).is_none())
        .map(|(slug, &id)| (id, slug.as_str()))
        .collect();
    new_cards.sort();
    // Vem do tamanho do lock (que é prefixo denso), não do maior id dele:
    // com o lock íntegro os dois coincidem, e com o lock truncado a diferença
    // é justamente o que se quer pegar.
    let mut next_free = snapshot.lock.entries.len() as u16;
    for (id, slug) in &new_cards {
        if *id != next_free {
            errors.push(format!(
                "`{slug}` tem id {id}, mas carta nova só anexa no fim: o próximo id livre é {next_free}"
            ));
        }
        next_free = id.saturating_add(1);
    }

    // 4. Nenhuma remoção, nenhuma reatribuição.
    for entry in &snapshot.lock.entries {
        match snapshot.registry.id_of(&entry.slug) {
            None => errors.push(format!(
                "`{}` desapareceu do registro: registro e lock são append-only",
                entry.slug
            )),
            Some(id) if id != entry.id => errors.push(format!(
                "`{}` mudou de id: {} -> {id}. Id não se reatribui.",
                entry.slug, entry.id
            )),
            Some(_) => {}
        }
    }

    // 5. Carta publicada não é editada.
    for card in &snapshot.cards {
        if let Some(entry) = snapshot.lock.get(&card.slug) {
            let actual = card::content_hash(card);
            if actual != entry.sha256 {
                errors.push(format!(
                    "carta publicada alterada: `{}` (hash {actual} não bate com {}). \
                     Carta publicada não se edita — crie outra carta e um novo conjunto.",
                    card.slug, entry.sha256
                ));
            }
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

/// Porta de CI. Não escreve nada.
pub fn run(root: &Path) -> Result<String, Vec<String>> {
    let snapshot = Snapshot::from_disk(root)?;
    let mut errors = check(&snapshot);

    for slug in snapshot.registry.by_slug.keys() {
        if snapshot.lock.get(slug).is_none() {
            errors.push(format!(
                "`{slug}` não está em cards/registry.lock: rode `generate` para publicar"
            ));
        }
    }

    if errors.is_empty() {
        let ordered = ordered(&snapshot)?;
        let expected = crate::generate::emit(&ordered);
        let path = crate::generate::generated_path(root);
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            Ok(_) => errors.push(format!(
                "{} está defasado: rode `cargo run -p vertice-tools -- generate`",
                path.display()
            )),
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let max_id = snapshot.registry.by_slug.values().copied().max().unwrap_or(0);
    Ok(format!(
        "{} cartas, ids 0..={max_id}, {} entradas no lock, geração em dia",
        snapshot.cards.len(),
        snapshot.lock.entries.len()
    ))
}

fn set_dirs(cards_dir: &Path, errors: &mut Vec<String>) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    let entries = match std::fs::read_dir(cards_dir) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(format!("{}: {e}", cards_dir.display()));
            return dirs;
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            // `flatten` aqui engoliria o erro e a carta sumiria sem aviso.
            errors.push(format!("{}: entrada de diretório ilegível", cards_dir.display()));
            continue;
        };
        let path = entry.path();
        let is_set = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("set-"));
        if is_set && path.is_dir() {
            dirs.push(path);
        }
    }
    // Ordem estável: nada aqui pode depender da ordem do sistema de arquivos.
    dirs.sort();
    dirs
}

fn card_files(set_dir: &Path, errors: &mut Vec<String>) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(set_dir) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(format!("{}: {e}", set_dir.display()));
            return files;
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            errors.push(format!("{}: entrada de diretório ilegível", set_dir.display()));
            continue;
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}
