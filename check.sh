#!/bin/bash
# Check the Rust engine against LSD on one model: for each of its configurations, run both with a
# computation trace and report "identical" or the first computation that differs.
# LSD runs single-threaded (-c 1): with threads, variables flagged parallel make its trace order vary.
# Usage: check.sh MODEL_DIR [STEPS]    e.g. check.sh Example/Test/RandomWalk 200
#   MODEL_DIR  model folder inside the LSD repo; the Rust model name is its last path component
#   STEPS      cap on MAX_STEP for the comparison (default 200)
# Env: LSD_REPO (default ./target/lsd-src, cloned if missing), WORK (default ./target/check),
#      RUST_BIN (default ./target/release/lsd-rs, built here; set it to check a model project's binary)
set -euo pipefail
cd "$(dirname "$0")"
MODEL_DIR=$1 STEPS=${2:-200}
LSD_REPO=${LSD_REPO:-$PWD/target/lsd-src} WORK=${WORK:-$PWD/target/check}
NAME=$(basename "$MODEL_DIR")

[ -n "${RUST_BIN:-}" ] || { cargo build --release -q; RUST_BIN=$PWD/target/release/lsd-rs; }
./oracle/build-oracle.sh "$LSD_REPO" "$MODEL_DIR" "$WORK" >/dev/null
fail=0
for cfg in "$WORK/$NAME"/*.lsd; do
  base=$(basename "$cfg" .lsd)
  [[ $base == _cap_* ]] && continue
  capped="$WORK/$NAME/_cap_$base.lsd"
  awk -v s="$STEPS" '/^MAX_STEP / { if ($2 > s) $2 = s } /^SEED 0$/ { $2 = 1 } { print }' "$cfg" > "$capped"   # LSD 8.1 rejects SEED 0
  seed=$(awk '/^SEED / { print $2 }' "$capped")
  (cd "$WORK/$NAME" && rm -f "_cap_${base}_"*.csv && LSD_TRACE=oracle_$base.trace ./lsdNW -f "_cap_$base.lsd" -s "$seed" -t -z -c 1 >/dev/null 2>"oracle_$base.err") \
    || { echo "$base: LSD failed: $(tail -3 "$WORK/$NAME/oracle_$base.err")"; fail=1; continue; }
  # run inside the model folder, like LSD, so files the model writes (e.g. networks) land next to LSD's
  if ! (cd "$WORK/$NAME" && "$RUST_BIN" run "$NAME" "$capped" --seed "$seed" --trace "$WORK/$NAME/rust_$base.trace" \
       --out "$WORK/$NAME/rust_out" >"$WORK/$NAME/rust_$base.json" 2>"$WORK/$NAME/rust_$base.err"); then
    echo "$base: Rust failed: $(head -c 600 "$WORK/$NAME/rust_$base.json")"
  fi
  python3 -I - "$WORK/$NAME/oracle_$base.trace" "$WORK/$NAME/rust_$base.trace" \
              "$WORK/$NAME/_cap_${base}_$seed.csv" "$WORK/$NAME/rust_out/_cap_${base}_$seed.csv" "$base" <<'EOF' || fail=1
import sys
a, b = (open(p).read().splitlines() for p in sys.argv[1:3])
name = sys.argv[5]
for i, (x, y) in enumerate(zip(a, b)):
    if x != y:
        print(f"{name}: DIFFERS at computation {i + 1} of {len(a)}")
        for j in range(max(0, i - 8), i):
            print(f"    same  {a[j]}")
        print(f"    LSD   {x}\n    Rust  {y}")
        sys.exit(1)
if len(a) != len(b):
    print(f"{name}: DIFFERS: LSD made {len(a)} computations, Rust {len(b)}; last common: {a[min(len(a), len(b)) - 1] if a and b else '-'}")
    sys.exit(1)
try:
    same_csv = open(sys.argv[3]).read() == open(sys.argv[4]).read()
except FileNotFoundError:
    same_csv = None
note = {True: "CSV identical", False: "CSV DIFFERS", None: "no CSV"}[same_csv]
print(f"{name}: identical ({len(a)} computations); {note}")
sys.exit(0 if same_csv is not False else 1)
EOF
done
exit $fail
