#!/bin/bash
# Re-prove every ported model against the original LSD (see check.sh). Prints one line per configuration.
# Usage: check-all.sh [STEPS]   (default: full length, except GameOfLife at 100 steps; about 10 minutes)
cd "$(dirname "$0")"
STEPS=${1:-1000000}
fail=0
for m in RandomWalk LogisticChaos ReplDynMacro Worm BugsLife; do
  ./check.sh Example/Test/$m "$STEPS" | sed "s/^/$m: /" || fail=1
done
./check.sh Example/Test/GameOfLife "$([ "$STEPS" -lt 100 ] && echo "$STEPS" || echo 100)" | sed "s/^/GameOfLife: /" || fail=1
./check.sh Example/SantAnna/K+S-original "$STEPS" | sed "s/^/K+S-original: /" || fail=1
for d in tests/lsd-models/*/; do
  m=$(basename "$d")
  ./check.sh "$PWD/$d" "$STEPS" | sed "s/^/$m: /" || fail=1
done
exit $fail
