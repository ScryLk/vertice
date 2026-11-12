use crate::ids::ObjectId;
use crate::state::Target;

/// Uma ação é um valor imutável e serializável: é o que o cliente envia,
/// o que o servidor valida e o que o log guarda. Não existe ação parcial.
///
/// Toda ação é `Copy` de propósito: o log de uma partida é `Vec<Action>`, e
/// é só isso que precisa ser persistido pra reproduzir o jogo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Baixa um terreno da mão. Limite de um por turno.
    PlayLand { object: ObjectId },
    /// Conjura da mão. `target` é obrigatório para efeitos que exigem alvo.
    Cast {
        object: ObjectId,
        target: Option<Target>,
    },
    /// Ataca um jogador ou criatura. Não existe etapa de bloqueio no M0:
    /// atacar uma criatura significa troca de dano; atacar o jogador é livre.
    Attack {
        attacker: ObjectId,
        target: Target,
    },
    /// Passa prioridade. Se houver mágica na pilha, resolve a do topo.
    Pass,
    /// Encerra o turno.
    EndTurn,
}
