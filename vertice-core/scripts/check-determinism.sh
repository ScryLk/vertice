#!/usr/bin/env bash
# Falha o CI se o core ganhar não-determinismo.
#
# A lista é curta e intencional: cada item é uma fonte de resultado que
# depende do ambiente, do relógio ou da ordem de hash. Todas quebram replay.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/src"

if [ ! -d "$SRC" ]; then
  echo "FALHA: diretório do motor não encontrado em $SRC" >&2
  exit 1
fi

# Padrões proibidos dentro do motor.
FORBIDDEN='thread_rng|rand::random|SystemTime|Instant::now|HashMap|HashSet|SystemRandom|getrandom'

if grep -RnE "$FORBIDDEN" "$SRC" --include='*.rs' | grep -vE '^\S+:[0-9]+: *//'; then
  echo
  echo "FALHA: não-determinismo detectado em vertice-core/src."
  echo "O motor não pode depender de relógio, entropia do sistema ou ordem de hash."
  echo "Injete RNG e tempo por parâmetro, e use BTreeMap/BTreeSet."
  exit 1
fi

echo "OK: vertice-core/src está determinístico."
