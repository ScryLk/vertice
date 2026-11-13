//! A curva de custo do M0.
//!
//! Referência: `.claude/agents/card-content.md` §2 — mágica de dano N custa
//! ⌈N/2⌉ (2 de dano por 1 mana é o teto); compra de N custa N; pump segue a
//! régua do dano. A parte de criatura foi recalibrada — ver abaixo.
//!
//! ## A base é a média do corpo, não o menor dos dois
//!
//! A primeira versão usava `min(P,T)`, e a escolha foi feita para acomodar uma
//! única carta publicada. Medida contra um corpus externo de 1.077 criaturas
//! baunilha (sem texto além de palavra-chave), essa base erra por **+1 mana na
//! mediana** nas criaturas sem palavra-chave: 62,9% das criaturas baunilha são
//! assimétricas e 28,4% têm diferença de 2 ou mais, e `min` cobra barato
//! justamente por elas.
//!
//! `⌈(P+T)/2⌉` fecha com 93,1% do corpus dentro da banda, contra 82,4% do
//! `min`. Nas 50 cartas publicadas quando a troca foi feita, **nenhuma** saiu
//! da banda — inclusive Sentinela de Baluarte (1/2 com vigilância por 1), que
//! era a razão de o `min` existir.
//!
//! ## O peso da palavra-chave é por palavra, e não é igual para todas
//!
//! O `+1` fixo valia o mesmo para uma e para três palavras. Medido, o
//! sobrecusto real é de ~½ mana por palavra, e desigual entre elas: Flying
//! cobra ~1, Vigilance e Deathtouch cobram ~0. Cobrar igual fazia a vigilância
//! pagar por voar, e cobrava de menos de quem só voa.
//!
//! **A tabela abaixo é a calibragem, e é obrigatória.** Palavra-chave nova
//! entra com peso medido; palavra-chave sem peso é palavra-chave sem preço, e
//! o compilador cobra isso no `match`.
//!
//! ## A banda
//!
//! ±1 mana. Passar disso é "fora da curva", e a carta então precisa de
//! `justificativa` no próprio arquivo — que é o escape que a referência já
//! prevê, e que **não entra no `content_hash`**: justificar não reescreve
//! carta publicada.
//!
//! Mudar a curva é mudar este arquivo. O validador não sabe nada de custo.

use crate::card::{Card, Effect, Keyword, Kind};

pub const BAND: i32 = 1;

/// Quanto uma palavra-chave soma ao corpo da criatura.
///
/// Números medidos contra um corpus externo de criaturas baunilha, não
/// intuição. Deathtouch empatar com Vigilance em zero surpreende, mas é o que
/// o dado diz: o corpo que carrega a palavra já pagou parte dela, e cobrar de
/// novo fazia toda criatura de toque letal sair caro.
fn keyword_weight(keyword: Keyword) -> i32 {
    match keyword {
        Keyword::Flying => 1,
        Keyword::Vigilance => 0,
        Keyword::Deathtouch => 0,
    }
}

/// Custo de referência para o custo efetivo da carta, ou `None` quando a
/// combinação não tem curva definida (hoje: mágica sem efeito, que é erro
/// estrutural e é pego antes).
pub fn reference(card: &Card) -> Option<i32> {
    match card.kind {
        Kind::Land => Some(0),
        Kind::Creature => {
            let corpo = ceil_half(i32::from(card.power) + i32::from(card.toughness));
            let palavras: i32 = card.keywords.iter().copied().map(keyword_weight).sum();
            // O gatilho de entrada é um efeito que a carta carrega: cobra o
            // mesmo que cobraria como mágica. Sem isso, criatura com gatilho
            // sairia pelo preço de criatura sem, e o validador estaria
            // certificando uma carta barata demais — o pior tipo de erro, o
            // que passa no portão.
            let gatilho = card.on_enter.and_then(effect_cost).unwrap_or(0);
            Some(corpo + palavras + gatilho)
        }
        Kind::Spell => effect_cost(card.effect),
    }
}

/// Quanto um efeito custa por si. Um efeito é um efeito, venha ele de mágica
/// conjurada ou de gatilho de entrada — régua única, senão o gatilho vira
/// desconto.
fn effect_cost(effect: Effect) -> Option<i32> {
    match effect {
        Effect::None => None,
        // Teto de 2 de dano por 1 mana.
        Effect::DealDamage(amount) => Some(ceil_half(i32::from(amount))),
        // Compra de N custa N.
        Effect::Draw(amount) => Some(i32::from(amount)),
        // Pump é dano em potencial, mesma régua do dano.
        Effect::Pump(power, toughness) => Some(ceil_half(
            i32::from(power).abs().max(i32::from(toughness).abs()),
        )),
        // Vida ganha espelha o dano causado. Não foi medida como a base da
        // criatura foi: ganhar vida é mais fraco que causar dano, e o corpus
        // tem menos cartas de vida pura para calibrar. Fica a régua do dano
        // até haver medida melhor — e é o primeiro número a revisar.
        Effect::GainLife(amount) => Some(ceil_half(i32::from(amount))),
    }
}

/// ⌈value / 2⌉, correto também para negativo.
///
/// `div_euclid` em vez de `/` porque `/` trunca em direção a zero e erra o
/// arredondamento de todo valor negativo par: `ceil(-2/2)` é `-1`, mas
/// `(-2 + 1) / 2` dá `0`. Hoje só entra valor não negativo aqui; a correção
/// existe para que o dia em que entrar negativo não seja um bug silencioso.
fn ceil_half(value: i32) -> i32 {
    (value + 1).div_euclid(2)
}
