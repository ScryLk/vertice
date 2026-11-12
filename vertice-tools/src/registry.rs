//! `cards/registry.toml` — a tabela `slug -> id` e a marca `next_id`.
//!
//! Este arquivo é a única fonte de `CardId`. Não é a posição da carta em
//! nenhuma lista: inserir uma carta no meio do *arquivo* é inofensivo, e
//! inserir no meio da *numeração* é recusado. É a diferença entre a
//! armadilha antiga (índice em array, que quebra replay em silêncio) e o
//! registro.
//!
//! **Por que `next_id` existe.** O `registry.lock` guarda o hash de cada
//! carta publicada, e daria para derivar "próximo id livre" dele. Não dá:
//! o lock é um arquivo, e apagar a entrada de uma carta apagaria junto a
//! prova de que ela existiu — o id voltaria a parecer livre. `next_id` é a
//! marca que sobrevive fora do lock. Ele **só sobe**, e subir é uma edição
//! explícita que aparece no diff. Com ele, apagar carta deixa de ser
//! invisível: o conjunto de ids do registro deixa de bater com a faixa
//! `0..next_id`.

use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Registry {
    /// Primeiro id nunca usado. Carta publicada tem id `< next_id`.
    pub next_id: u16,
    pub by_slug: BTreeMap<String, u16>,
}

impl Registry {
    pub fn parse(text: &str) -> Result<Self, String> {
        let table: toml::Table =
            toml::from_str(text).map_err(|e| format!("TOML inválido: {e}"))?;

        let mut by_slug = BTreeMap::new();
        let mut next_id = None;

        for (key, value) in &table {
            match key.as_str() {
                "next_id" => {
                    let toml::Value::Integer(raw) = value else {
                        return Err("`next_id` tem que ser inteiro".to_string());
                    };
                    next_id = Some(
                        u16::try_from(*raw)
                            .map_err(|_| format!("`next_id` fora da faixa 0..=65535: {raw}"))?,
                    );
                }
                "cards" => {
                    let toml::Value::Table(cards) = value else {
                        return Err("`cards` tem que ser uma tabela slug -> id".to_string());
                    };
                    for (slug, id) in cards {
                        let toml::Value::Integer(id) = id else {
                            return Err(format!("`{slug}`: id tem que ser inteiro"));
                        };
                        let id = u16::try_from(*id)
                            .map_err(|_| format!("`{slug}`: id fora da faixa 0..=65535: {id}"))?;
                        by_slug.insert(slug.clone(), id);
                    }
                }
                other => return Err(format!("campo desconhecido no registro: `{other}`")),
            }
        }

        let next_id = next_id.ok_or_else(|| {
            "`next_id` ausente: é a marca de que nenhuma carta publicada foi apagada".to_string()
        })?;

        if by_slug.is_empty() {
            return Err("registro vazio: nenhuma carta em `[cards]`".to_string());
        }

        Ok(Registry { next_id, by_slug })
    }

    pub fn id_of(&self, slug: &str) -> Option<u16> {
        self.by_slug.get(slug).copied()
    }
}
