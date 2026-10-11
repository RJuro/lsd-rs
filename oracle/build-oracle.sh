#!/bin/bash
# Build LSD's headless engine (lsdNW) for one model with pinned, CPU-independent flags,
# so its output can serve as the reference ("oracle") for a port.
#
# Usage: build-oracle.sh LSD_REPO MODEL_DIR OUT_DIR
#   LSD_REPO   checkout of github.com/marcov64/Lsd (cloned here if missing: src + Example only)
#   MODEL_DIR  model folder inside the repo, e.g. Example/Test/RandomWalk, or a path to your own model
#   OUT_DIR    where the model copy and its lsdNW binary go
set -euo pipefail
LSD=$1 MODEL=$2 OUT=$3
CXX=${CXX:-g++-15}   # GCC (libstdc++), not Apple clang: random draws depend on the C++ library

if [ ! -d "$LSD/src" ]; then
  # the full repo is 2.2 GB, mostly a bundled Windows toolchain; fetch only what lsdNW needs
  git clone --depth 1 --filter=blob:none --sparse https://github.com/marcov64/Lsd.git "$LSD"
  git -C "$LSD" sparse-checkout set src Example
fi
LSD=$(cd "$LSD" && pwd)
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)

# patched engine copy: write 17 significant digits instead of 10, so doubles round-trip exactly
if [ ! -d "$OUT/src-oracle" ]; then
  cp -R "$LSD/src" "$OUT/src-oracle"
  rm -f "$OUT/src-oracle"/*.o   # stale objects would hide the patch below
  sed -i.bak 's/#define SIG_DIG 10/#define SIG_DIG 17/' "$OUT/src-oracle/decl.h"
  rm "$OUT/src-oracle/decl.h.bak"
  perl -pi -e 's/^#include "decl.h"/#include <climits>\n#include "decl.h"/' "$OUT/src-oracle/util.cpp"   # INT_MAX, for GCC
  grep -q '#define SIG_DIG 17' "$OUT/src-oracle/decl.h" || { echo "SIG_DIG patch failed" >&2; exit 1; }
  # computation trace: with LSD_TRACE=file, log every computed value (c) and every write (w) in order
  perl -0pi -e 's/(#include "decl.h"\n)/$1\nstatic FILE *lsd_trace_f = getenv( "LSD_TRACE" ) ? fopen( getenv( "LSD_TRACE" ), "w" ) : NULL;\nvoid lsd_trace( const char *k, const char *lab, double v ) { if ( lsd_trace_f ) fprintf( lsd_trace_f, "%d\\t%s\\t%s\\t%.17g\\n", t, k, lab, v ); }\n/; s/(\n\tlast_update = t;\n)/$1\tlsd_trace( "c", label, val[ 0 ] );\n/' "$OUT/src-oracle/variab.cpp"
  perl -0pi -e 's/(#include "decl.h"\n)/$1\nvoid lsd_trace( const char *k, const char *lab, double v );\n/; s/(\t\tcv->val\[ eff_lag \] = value;\n\t\tcv->last_update = time;\n)/$1\t\tlsd_trace( "w", lab, value );\n/' "$OUT/src-oracle/object.cpp"
  [ "$(grep -c lsd_trace "$OUT/src-oracle/variab.cpp")" = 3 ] && [ "$(grep -c lsd_trace "$OUT/src-oracle/object.cpp")" = 2 ] \
    || { echo "trace patch failed" >&2; exit 1; }
fi

SRC_MODEL="$MODEL"; [ -d "$SRC_MODEL" ] || SRC_MODEL="$LSD/$MODEL"
M="$OUT/$(basename "$MODEL")"
rm -rf "$M" && cp -R "$SRC_MODEL" "$M"
{
  echo "SRC=$OUT/src-oracle"
  cat "$M/model_options.txt"
  echo
  echo "CC=$CXX"
  # no -march=native and no fused multiply-add: results must not depend on the CPU
  echo "GLOBAL_CC=-std=c++14 -w -ffp-contract=off"
  echo "PATH_HEADER=."
  echo "PATH_LIB=."
  echo "LIB=-lz -lpthread"
  cat "$LSD/src/makefile-NW.txt"
} > "$M/makefileNW"
ln -sfn src-oracle "$OUT/src"   # models that include "../src/fun_head.h"
# makefile-NW hard-codes g++ (Apple clang on macOS); force the GCC we pinned
make -C "$M" -f makefileNW CC_NW="$CXX" -j8 >/dev/null
echo "$M/lsdNW"
