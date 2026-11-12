//! `cards/registry.lock` — a memória de que carta publicada não muda.
//!
//! O registro (`registry.toml`) diz *qual* id cada carta tem. O lock diz
//! *quanto valia* o conteúdo de cada carta no momento em que foi publicada.
//! Sem ele, "carta publicada não é editada" é convenção; com ele, é portão.
//!
//! O lock também é append-only: `generate` só acrescenta entrada. Nunca
//! reescreve nem remove — se reescrevesse, ele mesmo apagaria a prova.

use crate::card::Card;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockEntry {
    pub slug: String,
    pub id: u16,
    pub sha256: String,
}

#[derive(Clone, Debug, Default)]
pub struct Lock {
    pub entries: Vec<LockEntry>,
}

impl Lock {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.trim().is_empty() {
            return Ok(Lock::default());
        }

        let table: toml::Table =
            toml::from_str(text).map_err(|e| format!("TOML inválido: {e}"))?;

        let mut entries = Vec::new();
        for (key, value) in &table {
            if key != "card" {
                return Err(format!("campo desconhecido no lock: `{key}`"));
            }
            let toml::Value::Array(items) = value else {
                return Err("`card` tem que ser uma lista de tabelas (`[[card]]`)".to_string());
            };
            for item in items {
                let toml::Value::Table(entry) = item else {
                    return Err("`[[card]]` tem que ser tabela".to_string());
                };
                let slug = match entry.get("slug") {
                    Some(toml::Value::String(s)) => s.clone(),
                    _ => return Err("entrada do lock sem `slug`".to_string()),
                };
                let id = match entry.get("id") {
                    Some(toml::Value::Integer(i)) => u16::try_from(*i)
                        .map_err(|_| format!("`{slug}`: id fora da faixa: {i}"))?,
                    _ => return Err(format!("`{slug}`: entrada do lock sem `id`")),
                };
                let sha256 = match entry.get("sha256") {
                    Some(toml::Value::String(s)) => s.clone(),
                    _ => return Err(format!("`{slug}`: entrada do lock sem `sha256`")),
                };
                entries.push(LockEntry { slug, id, sha256 });
            }
        }

        entries.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.slug.cmp(&b.slug)));
        Ok(Lock { entries })
    }

    pub fn get(&self, slug: &str) -> Option<&LockEntry> {
        self.entries.iter().find(|entry| entry.slug == slug)
    }

    pub fn max_id(&self) -> Option<u16> {
        self.entries.iter().map(|entry| entry.id).max()
    }

    /// Acrescenta a entrada de uma carta ainda não publicada. Devolve `false`
    /// se o slug já está no lock — nesse caso o chamador tem que comparar
    /// hash, não sobrescrever.
    pub fn append(&mut self, card: &Card, id: u16) -> bool {
        if self.get(&card.slug).is_some() {
            return false;
        }
        self.entries.push(LockEntry {
            slug: card.slug.clone(),
            id,
            sha256: crate::card::content_hash(card),
        });
        self.entries
            .sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.slug.cmp(&b.slug)));
        true
    }

    /// Serialização estável: mesma entrada, mesmos bytes, sempre.
    pub fn to_toml(&self) -> String {
        let mut out = String::from(
            "# Gerado por `vertice-tools generate`. Não edite à mão.\n\
             #\n\
             # APPEND-ONLY: nenhuma entrada é removida nem reescrita. A entrada de\n\
             # uma carta é o que prova que ela não mudou desde que foi publicada.\n",
        );
        for entry in &self.entries {
            out.push_str("\n[[card]]\n");
            out.push_str(&format!("slug = \"{}\"\n", entry.slug));
            out.push_str(&format!("id = {}\n", entry.id));
            out.push_str(&format!("sha256 = \"{}\"\n", entry.sha256));
        }
        out
    }
}
