//! O duelo offline, compilado para o navegador.
//!
//! É o mesmo motor: `vertice-core` não sabe que virou WASM. O que muda é quem
//! manda as ações — em vez de um WebSocket, a própria página.
//!
//! A superfície é JSON com a **mesma forma** que o servidor manda
//! (`PlayerViewDto`, `ActionDto`), de propósito: o React já sabe ler esses
//! tipos, então online e offline são o mesmo código do lado da tela. A única
//! diferença é quem produz a view.
//!
//! ## O que isto *não* é
//!
//! Não é o servidor rodando no cliente. Offline não tem autoridade nenhuma:
//! o jogador pode abrir o console e mentir. É modo de campanha e de treino,
//! não de partida valendo. Quem precisa de verdade é o servidor, e é por isso
//! que ele continua existindo.

use wasm_bindgen::prelude::*;

use vertice_bot::{choose_action, Level};
use vertice_core::cards::provisional_deck;
use vertice_core::rng::Rng;
use vertice_core::{apply, legal_actions, new_game, view, Action, GameState, PlayerId};
use vertice_protocol::{ActionDto, CardDefDto, PlayerViewDto};

/// O assento do humano. O bot fica no 1.
pub const HUMAN: PlayerId = 0;
const BOT: PlayerId = 1;

/// Teto de jogadas seguidas do bot, para um bug no bot não virar laço.
const BOT_TURN_LIMIT: usize = 512;

/// O duelo em si, sem nada de `wasm-bindgen`.
///
/// Existe separado do binding para poder ser testado nativo: o portão roda
/// nesta máquina, não num navegador.
pub struct Duel {
    state: GameState,
    level: Level,
    rng: Rng,
}

impl Duel {
    pub fn new(seed: u64, level: &str) -> Result<Self, String> {
        let level = match level {
            "calouro" => Level::Calouro,
            "formando" => Level::Formando,
            outro => return Err(format!("nível desconhecido: `{outro}`")),
        };

        let deck = provisional_deck(0);
        let state = new_game(&deck, &deck, seed).map_err(|error| format!("{error:?}"))?;

        let mut duel = Duel {
            state,
            level,
            rng: Rng::new(seed ^ 0xb07_b07_b07),
        };
        duel.run_bot();
        Ok(duel)
    }

    pub fn view(&self, seat: PlayerId) -> PlayerViewDto {
        PlayerViewDto::from(&view(&self.state, seat))
    }

    pub fn actions(&self, seat: PlayerId) -> Vec<ActionDto> {
        legal_actions(&self.state, seat)
            .into_iter()
            .map(ActionDto::from)
            .collect()
    }

    /// Aplica a ação do humano e deixa o bot responder.
    ///
    /// Recusa o que não estiver em `legal_actions` sem tocar no estado — a
    /// mesma regra do servidor, pelo mesmo motivo: só existe um validador de
    /// legalidade, e ele é o motor.
    pub fn act(&mut self, seat: PlayerId, action: &ActionDto) -> Result<(), String> {
        let action: Action = (*action).into();

        if !legal_actions(&self.state, seat).contains(&action) {
            return Err("ação ilegal".to_string());
        }

        apply(&mut self.state, action).map_err(|error| format!("{error:?}"))?;
        self.run_bot();
        Ok(())
    }

    pub fn fingerprint(&self) -> u64 {
        self.state.fingerprint()
    }

    pub fn is_over(&self) -> bool {
        self.state.winner.is_some()
    }

    fn run_bot(&mut self) {
        for _ in 0..BOT_TURN_LIMIT {
            if self.state.winner.is_some() {
                return;
            }

            let seat = self.state.priority;
            if seat != BOT {
                return;
            }

            let Some(action) = choose_action(&self.state, seat, self.level, &mut self.rng) else {
                return;
            };
            if apply(&mut self.state, action).is_err() {
                return;
            }
        }
    }
}

/// O catálogo inteiro, em JSON.
///
/// O cliente precisa disto para desenhar carta: nome, custo, corpo e efeito.
/// Sai por aqui, e não por uma cópia em TypeScript, porque o catálogo é dado
/// gerado — duas cópias divergiriam na primeira carta nova.
#[wasm_bindgen(js_name = catalogo)]
pub fn catalogo() -> String {
    let cartas: Vec<CardDefDto> = vertice_core::cards::CATALOG.iter().map(CardDefDto::from).collect();
    serde_json::to_string(&cartas).unwrap_or_else(|_| "[]".to_string())
}

/// O binding. Fino de propósito: só converte JSON e erro para o que o JS
/// entende. Toda a lógica está em [`Duel`], que é testável nativo.
#[wasm_bindgen(js_name = Duel)]
pub struct WasmDuel(Duel);

#[wasm_bindgen(js_class = Duel)]
impl WasmDuel {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: f64, level: &str) -> Result<WasmDuel, JsValue> {
        Duel::new(seed as u64, level)
            .map(WasmDuel)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = view)]
    pub fn view_json(&self, seat: u8) -> String {
        serde_json::to_string(&self.0.view(seat)).unwrap_or_else(|_| "null".to_string())
    }

    pub fn actions(&self, seat: u8) -> String {
        serde_json::to_string(&self.0.actions(seat)).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn act(&mut self, seat: u8, action_json: &str) -> Result<(), JsValue> {
        let action: ActionDto = serde_json::from_str(action_json)
            .map_err(|error| JsValue::from_str(&format!("ação malformada: {error}")))?;

        self.0
            .act(seat, &action)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn fingerprint(&self) -> String {
        self.0.fingerprint().to_string()
    }

    #[wasm_bindgen(js_name = isOver)]
    pub fn is_over(&self) -> bool {
        self.0.is_over()
    }
}
