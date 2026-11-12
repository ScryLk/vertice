//! Partida dourada: um registro congelado, em arquivo, com fingerprint fixo.
//!
//! É o único teste do repositório que não gera a própria entrada. A partida
//! vem de `tests/fixtures/partida-dourada.txt`, commitada, e o estado final
//! tem que bater bit a bit.
//!
//! Por que ele existe: `cards/registry.lock` guarda o hash de cada carta
//! publicada, mas é um arquivo — quem o reescrever reescreve a prova. Este
//! registro é a segunda testemunha, e ela não é derivada do lock: se uma carta
//! mudar de comportamento, a partida dourada joga diferente e o fingerprint
//! muda, mesmo que o lock tenha sido forjado.
//!
//! **Quando este teste falhar, a pergunta não é "como faço passar".** É "o que
//! mudou de comportamento e por quê". Se a mudança foi intencional, regenere:
//!
//! ```text
//! cargo test --test golden -- --ignored regerar_partida_dourada
//! ```
//!
//! e revise o diff do arquivo como parte do commit.

mod common;

use common::scripted_match;
use vertice_core::*;

const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/partida-dourada.txt"
);

/// Semente e caos do gerador. Fazem parte da partida: mudar qualquer um
/// muda o registro inteiro.
const SEED: u64 = 20_260_119;
const CHAOS: u64 = 99;

const HEADER: &str = "\
# Partida dourada do Vértice — registro congelado do M1.2.
#
# Este arquivo É a partida. Se o motor mudar comportamento, tests/golden.rs
# falha e aponta para cá. Não edite à mão e não regenere só para ficar verde:
# a diferença é o bug.
#
# Regerar (somente quando a mudança de comportamento for intencional):
#   cargo test --test golden -- --ignored regerar_partida_dourada
#
# Formato, um item por linha:
#   engine_version <str>       versão do motor no momento do registro
#   card_set_version <str>     versão do conjunto de cartas
#   seed <u64>                 semente do embaralhamento
#   deck0 <ids>                ids separados por espaço
#   deck1 <ids>
#   hand0 <ids>                opcional; ausente = mão normal
#   hand1 <ids>
#   actions <n>                seguido de exatamente n linhas de ação
#   fingerprint <u64>          estado final tem que bater com este
#
# Ação:
#   land:<obj>        PlayLand
#   cast:<obj>        Cast sem alvo
#   cast:<obj>:p<id>  Cast com alvo jogador
#   cast:<obj>:o<id>  Cast com alvo objeto
#   atk:<obj>:p<id>   Attack jogador
#   atk:<obj>:o<id>   Attack objeto
#   pass              Pass
#   end               EndTurn

";

struct Golden {
    engine_version: String,
    card_set_version: String,
    seed: u64,
    deck0: Vec<CardId>,
    deck1: Vec<CardId>,
    hand0: Vec<CardId>,
    hand1: Vec<CardId>,
    actions: Vec<Action>,
    fingerprint: u64,
}

#[test]
fn partida_dourada_reproduz_bit_a_bit() {
    let golden = load();
    let record = build_record(&golden);
    let state = replay(&record).expect("registro dourado tem que reproduzir sem erro");

    assert_eq!(
        state.fingerprint(),
        golden.fingerprint,
        "a partida dourada jogou diferente. Algo mudou de comportamento — \
         carta, regra ou ordem de `legal_actions`. Descubra o quê antes de regerar."
    );
}

#[test]
fn versoes_do_registro_batem_com_o_codigo() {
    // O registro não reproduz pela versão — ele reproduz pelo comportamento.
    // Estas duas checagens existem para que mudar o conjunto seja uma decisão
    // consciente, e não algo que passa raspando.
    let golden = load();
    assert_eq!(
        golden.engine_version, ENGINE_VERSION,
        "ENGINE_VERSION mudou desde o registro dourado"
    );
    assert_eq!(
        golden.card_set_version, CARD_SET_VERSION,
        "CARD_SET_VERSION mudou desde o registro dourado: o conjunto é outro, \
         então a partida dourada precisa ser regerada e revisada"
    );
}

#[test]
#[ignore = "regera a fixture dourada; só quando a mudança de comportamento for intencional"]
fn regerar_partida_dourada() {
    let record = scripted_match(SEED, CHAOS);
    let state = replay(&record).expect("partida gerada tem que reproduzir");
    if let Some(parent) = std::path::Path::new(FIXTURE_PATH).parent() {
        std::fs::create_dir_all(parent).expect("criar o diretório da fixture");
    }
    std::fs::write(FIXTURE_PATH, render(&record, state.fingerprint()))
        .expect("escrever a fixture dourada");
}

fn load() -> Golden {
    let text = std::fs::read_to_string(FIXTURE_PATH).unwrap_or_else(|e| {
        panic!(
            "não consegui ler {FIXTURE_PATH}: {e}\n\
             Regenere com: cargo test --test golden -- --ignored regerar_partida_dourada"
        )
    });
    parse(&text).unwrap_or_else(|e| panic!("{FIXTURE_PATH} malformado: {e}"))
}

fn build_record(golden: &Golden) -> MatchRecord {
    let mut record = if golden.hand0.is_empty() && golden.hand1.is_empty() {
        MatchRecord::new(golden.deck0.clone(), golden.deck1.clone(), golden.seed)
    } else {
        MatchRecord::with_hands(
            golden.deck0.clone(),
            golden.deck1.clone(),
            golden.seed,
            golden.hand0.clone(),
            golden.hand1.clone(),
        )
    };
    record.actions = golden.actions.clone();
    record
}

fn parse(text: &str) -> Result<Golden, String> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    let mut engine_version = None;
    let mut card_set_version = None;
    let mut seed = None;
    let mut deck0 = None;
    let mut deck1 = None;
    let mut hand0 = Vec::new();
    let mut hand1 = Vec::new();
    let mut actions = None;
    let mut fingerprint = None;

    let mut index = 0;
    while index < lines.len() {
        let (key, rest) = match lines[index].split_once(' ') {
            Some((key, rest)) => (key, rest.trim()),
            None => (lines[index], ""),
        };

        match key {
            "engine_version" => engine_version = Some(rest.to_string()),
            "card_set_version" => card_set_version = Some(rest.to_string()),
            "seed" => seed = Some(parse_number(rest, "seed")?),
            "deck0" => deck0 = Some(parse_ids(rest)?),
            "deck1" => deck1 = Some(parse_ids(rest)?),
            "hand0" => hand0 = parse_ids(rest)?,
            "hand1" => hand1 = parse_ids(rest)?,
            "fingerprint" => fingerprint = Some(parse_number(rest, "fingerprint")?),
            "actions" => {
                let count: usize = parse_number(rest, "actions")?;
                let mut parsed = Vec::with_capacity(count);
                for offset in 0..count {
                    let line = lines.get(index + 1 + offset).ok_or_else(|| {
                        format!("declara {count} ações, mas o arquivo acaba antes")
                    })?;
                    parsed.push(decode_action(line)?);
                }
                index += count;
                actions = Some(parsed);
            }
            other => return Err(format!("chave desconhecida: `{other}`")),
        }

        index += 1;
    }

    Ok(Golden {
        engine_version: engine_version.ok_or("falta `engine_version`")?,
        card_set_version: card_set_version.ok_or("falta `card_set_version`")?,
        seed: seed.ok_or("falta `seed`")?,
        deck0: deck0.ok_or("falta `deck0`")?,
        deck1: deck1.ok_or("falta `deck1`")?,
        hand0,
        hand1,
        actions: actions.ok_or("falta `actions`")?,
        fingerprint: fingerprint.ok_or("falta `fingerprint`")?,
    })
}

fn parse_number<T: std::str::FromStr>(text: &str, key: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|e| format!("`{key}` inválido (`{text}`): {e}"))
}

fn parse_ids(text: &str) -> Result<Vec<CardId>, String> {
    text.split_whitespace()
        .map(|token| parse_number(token, "id de carta"))
        .collect()
}

fn decode_action(token: &str) -> Result<Action, String> {
    let parts: Vec<&str> = token.split(':').collect();
    match parts.as_slice() {
        ["pass"] => Ok(Action::Pass),
        ["end"] => Ok(Action::EndTurn),
        ["land", object] => Ok(Action::PlayLand {
            object: decode_object(object)?,
        }),
        ["cast", object] => Ok(Action::Cast {
            object: decode_object(object)?,
            target: None,
        }),
        ["cast", object, target] => Ok(Action::Cast {
            object: decode_object(object)?,
            target: Some(decode_target(target)?),
        }),
        ["atk", attacker, target] => Ok(Action::Attack {
            attacker: decode_object(attacker)?,
            target: decode_target(target)?,
        }),
        _ => Err(format!("ação malformada: `{token}`")),
    }
}

fn decode_object(text: &str) -> Result<ObjectId, String> {
    parse_number(text, "ObjectId").map(ObjectId)
}

fn decode_target(text: &str) -> Result<Target, String> {
    if let Some(player) = text.strip_prefix('p') {
        return parse_number(player, "jogador").map(Target::Player);
    }
    if let Some(object) = text.strip_prefix('o') {
        return parse_number(object, "objeto").map(|id| Target::Object(ObjectId(id)));
    }
    Err(format!("alvo malformado: `{text}` (esperado p<jogador> ou o<objeto>)"))
}

fn encode_action(action: Action) -> String {
    fn target_text(target: Target) -> String {
        match target {
            Target::Player(player) => format!("p{player}"),
            Target::Object(object) => format!("o{}", object.0),
        }
    }

    match action {
        Action::PlayLand { object } => format!("land:{}", object.0),
        Action::Cast {
            object,
            target: None,
        } => format!("cast:{}", object.0),
        Action::Cast {
            object,
            target: Some(target),
        } => format!("cast:{}:{}", object.0, target_text(target)),
        Action::Attack { attacker, target } => {
            format!("atk:{}:{}", attacker.0, target_text(target))
        }
        Action::Pass => "pass".to_string(),
        Action::EndTurn => "end".to_string(),
    }
}

fn render(record: &MatchRecord, fingerprint: u64) -> String {
    let ids = |list: &[CardId]| {
        list.iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };

    let mut out = String::from(HEADER);
    out.push_str(&format!("engine_version {}\n", record.engine_version));
    out.push_str(&format!("card_set_version {}\n", record.card_set_version));
    out.push_str(&format!("seed {}\n", record.seed));
    out.push_str(&format!("deck0 {}\n", ids(&record.deck0)));
    out.push_str(&format!("deck1 {}\n", ids(&record.deck1)));
    if !record.hand0.is_empty() || !record.hand1.is_empty() {
        out.push_str(&format!("hand0 {}\n", ids(&record.hand0)));
        out.push_str(&format!("hand1 {}\n", ids(&record.hand1)));
    }
    out.push_str(&format!("actions {}\n", record.actions.len()));
    for action in &record.actions {
        out.push_str(&encode_action(*action));
        out.push('\n');
    }
    out.push_str(&format!("fingerprint {fingerprint}\n"));
    out
}
