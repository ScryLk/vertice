//! Uma carta em TOML: leitura e verificação estrutural.
//!
//! O parser é manual de propósito. O produto desta ferramenta é *diagnóstico*,
//! e o validador precisa apontar o campo errado pelo nome. Um derive de
//! `serde` daria mensagem pior e esconderia as regras de estrutura (criatura
//! com resistência 0, terreno com custo > 0) em algum lugar implícito.

use crate::curve;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Land,
    Creature,
    Spell,
}

impl Kind {
    pub fn token(self) -> &'static str {
        match self {
            Kind::Land => "land",
            Kind::Creature => "creature",
            Kind::Spell => "spell",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "land" => Some(Kind::Land),
            "creature" => Some(Kind::Creature),
            "spell" => Some(Kind::Spell),
            _ => None,
        }
    }
}

/// Lista fechada. Palavra-chave nova exige definição, implementação em
/// `engine.rs` e teste — por isso o validador recusa o que não está aqui.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Keyword {
    Flying,
    Vigilance,
    Deathtouch,
}

impl Keyword {
    pub fn token(self) -> &'static str {
        match self {
            Keyword::Flying => "flying",
            Keyword::Vigilance => "vigilance",
            Keyword::Deathtouch => "deathtouch",
        }
    }

    pub fn rust_name(self) -> &'static str {
        match self {
            Keyword::Flying => "Flying",
            Keyword::Vigilance => "Vigilance",
            Keyword::Deathtouch => "Deathtouch",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "flying" => Some(Keyword::Flying),
            "vigilance" => Some(Keyword::Vigilance),
            "deathtouch" => Some(Keyword::Deathtouch),
            _ => None,
        }
    }

    pub const ALL: [Keyword; 3] = [Keyword::Flying, Keyword::Vigilance, Keyword::Deathtouch];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    None,
    DealDamage(u8),
    Draw(u8),
    Pump(i8, i8),
    GainLife(u8),
    /// O oponente do controlador perde N de vida. Sem alvo.
    LoseLife(u8),
}

impl Effect {
    /// Forma canônica: usada no hash do registro, não na apresentação.
    pub fn token(self) -> String {
        match self {
            Effect::None => "none".to_string(),
            Effect::DealDamage(n) => format!("deal_damage:{n}"),
            Effect::Draw(n) => format!("draw:{n}"),
            Effect::Pump(p, t) => format!("pump:{p},{t}"),
            Effect::GainLife(n) => format!("gain_life:{n}"),
            Effect::LoseLife(n) => format!("lose_life:{n}"),
        }
    }

    /// Espelha `Effect::needs_target` do motor. Duas definições, um conceito:
    /// o validador recusa gatilho com alvo justamente porque o motor não
    /// consegue resolver escolha dentro de um gatilho.
    pub fn needs_target(self) -> bool {
        matches!(self, Effect::DealDamage(_) | Effect::Pump(..))
    }
}

#[derive(Clone, Debug)]
pub struct Card {
    pub slug: String,
    pub name: String,
    pub cost: u8,
    pub kind: Kind,
    pub power: i8,
    pub toughness: i8,
    pub keywords: Vec<Keyword>,
    pub effect: Effect,
    /// Efeito que dispara quando a criatura entra no campo.
    pub on_enter: Option<Effect>,
    /// Efeito que dispara quando a criatura morre.
    pub on_die: Option<Effect>,
    /// Escape previsto pelo `card-content`: carta fora da curva só passa com
    /// uma linha dizendo por quê. Não entra no hash — é comentário humano.
    pub justificativa: Option<String>,
}

/// Campos aceitos. Qualquer outro é erro: erro de digitação silencioso em
/// dado de jogo é bug que aparece semanas depois.
const ALLOWED: [&str; 11] = [
    "slug",
    "name",
    "cost",
    "type",
    "power",
    "toughness",
    "keywords",
    "effect",
    "on_enter",
    "on_die",
    "justificativa",
];

pub fn parse(text: &str) -> Result<Card, String> {
    let mut table: toml::Table =
        toml::from_str(text).map_err(|e| format!("TOML inválido: {e}"))?;

    let slug = take_string(&mut table, "slug")?;
    let name = take_string(&mut table, "name")?;
    let cost = take_u8(&mut table, "cost")?;

    let raw_kind = take_string(&mut table, "type")?;
    let kind = Kind::parse(&raw_kind).ok_or_else(|| {
        format!("`type` desconhecido: `{raw_kind}` (esperado land, creature ou spell)")
    })?;

    let power = take_i8_or(&mut table, "power", 0)?;
    let toughness = take_i8_or(&mut table, "toughness", 0)?;
    let keywords = take_keywords(&mut table)?;
    let effect = take_effect(&mut table)?;
    let on_enter = take_optional_effect(&mut table, "on_enter")?;
    let on_die = take_optional_effect(&mut table, "on_die")?;
    let justificativa = take_optional_string(&mut table, "justificativa")?;

    if let Some(extra) = table.keys().next() {
        return Err(format!(
            "campo desconhecido: `{extra}` (aceitos: {})",
            ALLOWED.join(", ")
        ));
    }

    Ok(Card {
        slug,
        name,
        cost,
        kind,
        power,
        toughness,
        keywords,
        effect,
        on_enter,
        on_die,
        justificativa,
    })
}

/// Identidade do conteúdo de uma carta, para o `registry.lock`.
///
/// Hasheia a forma *semântica*, não os bytes do arquivo: reformatar ou
/// reordenar palavras-chave não é alterar a carta. Mudar custo, força,
/// resistência, tipo, nome ou efeito é.
pub fn content_hash(card: &Card) -> String {
    use sha2::{Digest, Sha256};

    let mut keywords: Vec<&str> = card.keywords.iter().map(|k| k.token()).collect();
    keywords.sort_unstable();

    // Separador 0x1F: não aparece em nome de carta, então não há como dois
    // campos diferentes colidirem por concatenação.
    let mut canonical = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
        card.slug,
        card.name,
        card.cost,
        card.kind.token(),
        card.power,
        card.toughness,
        keywords.join(","),
        card.effect.token(),
    );

    // O gatilho entra **só quando existe**. Acrescentar um campo fixo mudaria
    // o hash de toda carta já publicada e o validador as acusaria de
    // alteradas — um commit de motor reescrevendo o lock inteiro. Ausência
    // continua distinguível de presença (uma não tem o separador), que é
    // tudo o que o lock precisa garantir.
    if let Some(trigger) = card.on_enter {
        canonical.push('\u{1f}');
        canonical.push_str(&trigger.token());
    }

    // `on_die` vai **rotulado**, e a assimetria é dívida herdada de propósito.
    // O slot de `on_enter` está no formato desde a publicação das primeiras
    // cartas com gatilho e não pode ganhar rótulo sem reescrever o lock delas.
    // `on_die` nasce agora, então nasce rotulado — sem isso, "tem gatilho de
    // entrada de compra 1" e "tem gatilho de morte de compra 1" produziriam a
    // mesma string canônica, e publicar um e depois trocar pelo outro passaria
    // batido.
    if let Some(trigger) = card.on_die {
        canonical.push('\u{1f}');
        canonical.push_str("on_die:");
        canonical.push_str(&trigger.token());
    }

    let digest = Sha256::digest(canonical.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Verificações que valem para qualquer carta, independente do conjunto.
pub fn validate(card: &Card) -> Vec<String> {
    let mut errors = Vec::new();
    let at = |msg: String| format!("`{}`: {msg}", card.slug);

    if card.name.trim().is_empty() {
        errors.push(at("nome vazio".into()));
    }
    if !is_slug(&card.slug) {
        errors.push(at(
            "slug inválido (só minúscula, dígito e hífen, sem hífen dobrado)".into(),
        ));
    }

    let mut seen = std::collections::BTreeSet::new();
    for keyword in &card.keywords {
        if !seen.insert(keyword.token()) {
            errors.push(at(format!("palavra-chave repetida: `{}`", keyword.token())));
        }
    }

    match card.kind {
        Kind::Land => {
            if card.cost != 0 {
                errors.push(at(format!("terreno não é conjurado: custo tem que ser 0, veio {}", card.cost)));
            }
            if card.power != 0 || card.toughness != 0 {
                errors.push(at(format!(
                    "terreno não tem corpo: força/resistência têm que ser 0/0, veio {}/{}",
                    card.power, card.toughness
                )));
            }
            if card.effect != Effect::None {
                // O motor recusa conjurar terreno (`cast` devolve
                // IllegalAction). Efeito em terreno seria texto que o motor
                // ignora — a divergência que o `card-content` existe pra evitar.
                errors.push(at("terreno com efeito: o motor não aplica".into()));
            }
            for (campo, gatilho) in [("on_enter", card.on_enter), ("on_die", card.on_die)] {
                if gatilho.is_some() {
                    errors.push(at(format!(
                        "terreno com `{campo}`: só criatura dispara gatilho no motor"
                    )));
                }
            }
        }
        Kind::Creature => {
            if card.power < 1 {
                errors.push(at(format!("criatura com força {}: mínimo é 1", card.power)));
            }
            if card.toughness < 1 {
                errors.push(at(format!(
                    "criatura com resistência {}: morreria ao entrar, mínimo é 1",
                    card.toughness
                )));
            }
            if card.effect != Effect::None {
                errors.push(at("criatura com efeito: a DSL do M1 só dá efeito a mágica".into()));
            }
            if card.cost == 0 {
                errors.push(at("criatura de custo 0".into()));
            }
            for (campo, gatilho) in [("on_enter", card.on_enter), ("on_die", card.on_die)] {
                match gatilho {
                    None => {}
                    Some(Effect::None) => errors.push(at(format!(
                        "`{campo}` sem efeito: o campo só existe quando há efeito"
                    ))),
                    // O corte da fatia, escrito onde alguém vai esbarrar nele:
                    // escolher alvo no meio de um gatilho exige que o gatilho
                    // use a pilha, e gatilho usar a pilha é a fatia seguinte.
                    Some(effect) if effect.needs_target() => errors.push(at(format!(
                        "`{campo}` com efeito que exige alvo (`{}`): sem alvo não há escolha, \
                         e é isso que permite o gatilho resolver na hora. Gatilho com alvo é \
                         fatia própria",
                        effect.token()
                    ))),
                    Some(_) => {}
                }
            }
        }
        Kind::Spell => {
            if card.power != 0 || card.toughness != 0 {
                errors.push(at(format!(
                    "mágica não tem corpo: força/resistência têm que ser 0/0, veio {}/{}",
                    card.power, card.toughness
                )));
            }
            if card.effect == Effect::None {
                errors.push(at("mágica sem efeito".into()));
            }
            if !card.keywords.is_empty() {
                errors.push(at("mágica com palavra-chave: o motor não aplica".into()));
            }
            if card.cost == 0 {
                errors.push(at("mágica de custo 0".into()));
            }
            for (campo, gatilho) in [("on_enter", card.on_enter), ("on_die", card.on_die)] {
                if gatilho.is_some() {
                    errors.push(at(format!(
                        "mágica com gatilho (`{campo}`): mágica nem entra no campo nem morre"
                    )));
                }
            }
        }
    }

    if let Some(reference) = curve::reference(card) {
        let delta = card.cost as i32 - reference;
        if delta.abs() > curve::BAND && card.justificativa.is_none() {
            errors.push(at(format!(
                "custo {} fora da curva (referência {reference}, banda ±{}): justifique em `justificativa`",
                card.cost,
                curve::BAND
            )));
        }
    }

    errors
}

fn is_slug(slug: &str) -> bool {
    !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn take(table: &mut toml::Table, key: &str) -> Option<toml::Value> {
    table.remove(key)
}

fn type_name(value: &toml::Value) -> &'static str {
    match value {
        toml::Value::String(_) => "string",
        toml::Value::Integer(_) => "inteiro",
        toml::Value::Float(_) => "float",
        toml::Value::Boolean(_) => "booleano",
        toml::Value::Datetime(_) => "data/hora",
        toml::Value::Array(_) => "lista",
        toml::Value::Table(_) => "tabela",
    }
}

fn required(table: &mut toml::Table, key: &str) -> Result<toml::Value, String> {
    take(table, key).ok_or_else(|| format!("campo obrigatório ausente: `{key}`"))
}

fn take_string(table: &mut toml::Table, key: &str) -> Result<String, String> {
    match required(table, key)? {
        toml::Value::String(s) => Ok(s),
        other => Err(format!("`{key}`: esperado string, veio {}", type_name(&other))),
    }
}

fn take_optional_string(table: &mut toml::Table, key: &str) -> Result<Option<String>, String> {
    match take(table, key) {
        None => Ok(None),
        Some(toml::Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(toml::Value::String(s)) => Ok(Some(s)),
        Some(other) => Err(format!("`{key}`: esperado string, veio {}", type_name(&other))),
    }
}

fn take_integer(table: &mut toml::Table, key: &str) -> Result<i64, String> {
    match required(table, key)? {
        toml::Value::Integer(i) => Ok(i),
        other => Err(format!("`{key}`: esperado inteiro, veio {}", type_name(&other))),
    }
}

fn take_u8(table: &mut toml::Table, key: &str) -> Result<u8, String> {
    let value = take_integer(table, key)?;
    u8::try_from(value).map_err(|_| format!("`{key}` fora da faixa 0..=255: {value}"))
}

fn take_i8_or(table: &mut toml::Table, key: &str, default: i8) -> Result<i8, String> {
    match take(table, key) {
        None => Ok(default),
        Some(toml::Value::Integer(i)) => {
            i8::try_from(i).map_err(|_| format!("`{key}` fora da faixa -128..=127: {i}"))
        }
        Some(other) => Err(format!("`{key}`: esperado inteiro, veio {}", type_name(&other))),
    }
}

fn take_keywords(table: &mut toml::Table) -> Result<Vec<Keyword>, String> {
    let value = match take(table, "keywords") {
        None => return Ok(Vec::new()),
        Some(value) => value,
    };
    let toml::Value::Array(items) = value else {
        return Err(format!("`keywords`: esperado lista, veio {}", type_name(&value)));
    };

    let mut keywords = Vec::with_capacity(items.len());
    for item in items {
        let toml::Value::String(token) = item else {
            return Err(format!("`keywords`: esperado string na lista, veio {}", type_name(&item)));
        };
        let keyword = Keyword::parse(&token).ok_or_else(|| {
            let known: Vec<&str> = Keyword::ALL.iter().map(|k| k.token()).collect();
            format!("palavra-chave desconhecida: `{token}` (conhecidas: {})", known.join(", "))
        })?;
        keywords.push(keyword);
    }
    Ok(keywords)
}

fn take_effect(table: &mut toml::Table) -> Result<Effect, String> {
    parse_effect("effect", required(table, "effect")?)
}

/// `on_enter` é opcional: ausente significa "a carta não tem gatilho".
///
/// Aceita a mesma forma de tabela que `effect` de propósito — um efeito é um
/// efeito, e ter duas sintaxes para a mesma coisa faria o autor de carta
/// errar por motivo nenhum.
fn take_optional_effect(table: &mut toml::Table, key: &str) -> Result<Option<Effect>, String> {
    match take(table, key) {
        None => Ok(None),
        Some(value) => parse_effect(key, value).map(Some),
    }
}

fn parse_effect(key: &str, value: toml::Value) -> Result<Effect, String> {
    let toml::Value::Table(mut effect) = value else {
        return Err(format!(
            "`{key}`: esperado tabela com `kind`, veio {}",
            type_name(&value)
        ));
    };

    let raw = take_string(&mut effect, "kind")?;
    let parsed = match raw.as_str() {
        "none" => Effect::None,
        "deal_damage" => Effect::DealDamage(take_u8(&mut effect, "amount")?),
        "draw" => Effect::Draw(take_u8(&mut effect, "amount")?),
        "gain_life" => Effect::GainLife(take_u8(&mut effect, "amount")?),
        "lose_life" => Effect::LoseLife(take_u8(&mut effect, "amount")?),
        "pump" => {
            let power = take_i8_or(&mut effect, "power", 0)?;
            let toughness = take_i8_or(&mut effect, "toughness", 0)?;
            Effect::Pump(power, toughness)
        }
        other => {
            return Err(format!(
                "`{key}.kind` desconhecido: `{other}` (esperado none, deal_damage, draw, \
                 gain_life, lose_life ou pump)"
            ))
        }
    };

    if let Some(extra) = effect.keys().next() {
        return Err(format!("`{key}`: campo desconhecido `{extra}`"));
    }

    Ok(parsed)
}
