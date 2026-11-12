//! RNG determinístico e serializável.
//!
//! xorshift64* — pequeno, rápido, sem dependência e, o mais importante,
//! o estado inteiro cabe em um `u64`. Isso significa que ele entra no
//! `GameState`, entra na comparação de replay, e não existe estado oculto
//! em lugar nenhum.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Seed zero trava o xorshift; nunca deixe acontecer.
        Self {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Inteiro em `[0, n)`. Pânico se `n == 0`.
    pub fn range(&mut self, n: usize) -> usize {
        assert!(n > 0, "range(0) não tem resultado possível");
        (self.next_u64() % n as u64) as usize
    }

    /// Fisher-Yates. Determinístico por construção, dado o estado do RNG.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.range(i + 1);
            items.swap(i, j);
        }
    }

    /// Exposto para o teste de determinismo e para auditoria de replay.
    pub fn state(&self) -> u64 {
        self.state
    }
}
