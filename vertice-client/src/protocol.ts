// Espelho do protocolo escrito à mão em `vertice-server/src/protocol/`.
//
// Este arquivo é a outra ponta do contrato: o servidor traduz os tipos do
// motor para JSON, e aqui o JSON volta a ser tipo. Quando o servidor mudar a
// forma, `PROTOCOL_VERSION` sobe e este arquivo tem que acompanhar — é o
// preço, e é menor que o de ter `serde` dentro do motor.
//
// Nada aqui decide regra. A única coisa que o cliente escolhe é *qual* ação
// mandar, e o servidor recusa se ela não estiver em `legal_actions`.

export const PROTOCOL_VERSION = 2;

export type PlayerId = number;
export type CardId = number;

export type TargetDto =
  | { type: 'player'; player: PlayerId }
  | { type: 'object'; object: number };

export type ActionDto =
  | { type: 'play_land'; object: number }
  | { type: 'cast'; object: number; target: TargetDto | null }
  | { type: 'attack'; attacker: number; target: TargetDto }
  | { type: 'pass' }
  | { type: 'end_turn' };

export type KeywordDto = 'flying' | 'vigilance' | 'deathtouch';
export type PhaseDto = 'main' | 'reaction';

export interface CardViewDto {
  object: number;
  card: number;
  name: string;
  power: number;
  toughness: number;
  tapped: boolean;
  damage: number;
  keywords: KeywordDto[];
}

export interface OpponentViewDto {
  id: PlayerId;
  life: number;
  hand_count: number;
  library_count: number;
  battlefield: CardViewDto[];
  graveyard: number[];
}

export interface SelfViewDto {
  id: PlayerId;
  life: number;
  hand: CardViewDto[];
  library_count: number;
  battlefield: CardViewDto[];
  graveyard: number[];
  mana: number;
}

export interface StackViewDto {
  controller: PlayerId;
  name: string;
}

export interface PlayerViewDto {
  you: SelfViewDto;
  opponent: OpponentViewDto;
  turn: number;
  phase: PhaseDto;
  priority: PlayerId;
  stack: StackViewDto[];
  legal_actions: ActionDto[];
  winner: PlayerId | null;
}

export type RejectReason = 'illegal_action' | 'game_over' | 'stale';
export type ErrorKind = 'protocol_mismatch' | 'unknown_match' | 'seat_taken';

export type ServerMessage =
  | { type: 'welcome'; protocol: number; seat: PlayerId; seq: number; view: PlayerViewDto }
  | { type: 'view'; seq: number; view: PlayerViewDto }
  | { type: 'rejected'; seq: number; reason: RejectReason }
  | { type: 'error'; kind: ErrorKind };

export type ClientMessage =
  | { type: 'join'; protocol: number; seat: PlayerId }
  | { type: 'action'; seq: number; action: ActionDto }
  | { type: 'resync'; seq: number };

export interface CreateMatchResponse {
  match_id: string;
  protocol: number;
}

const NOMES_DE_PALAVRA_CHAVE: Record<KeywordDto, string> = {
  flying: 'voar',
  vigilance: 'vigilância',
  deathtouch: 'toque letal',
};

export function nomeDaPalavraChave(keyword: KeywordDto): string {
  return NOMES_DE_PALAVRA_CHAVE[keyword];
}

// ---------------------------------------------------------------- catálogo
//
// A carta fora do motor. Vem de `CardDefDto` no `vertice-protocol`, servido
// pelo WASM em `catalogo()`.

export type CardKindDto = 'land' | 'creature' | 'spell';

export type EffectDto =
  | { kind: 'none' }
  | { kind: 'deal_damage'; amount: number }
  | { kind: 'draw'; amount: number }
  | { kind: 'pump'; power: number; toughness: number }
  | { kind: 'gain_life'; amount: number };

export interface CardDefDto {
  id: CardId;
  name: string;
  cost: number;
  kind: CardKindDto;
  power: number;
  toughness: number;
  keywords: KeywordDto[];
  effect: EffectDto;
  /** Gatilho de entrada. O servidor manda `null` quando não há. */
  on_enter: EffectDto | null;
}
