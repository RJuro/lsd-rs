# Validating a port against LSD

Two tiers. **Exact:** the port reproduces LSD bit for bit, computation by computation. **Statistical:** for deliberate
changes that can't match bit for bit, the port reproduces LSD's distributions of outcomes.

## How it runs

```bash
./check.sh Example/Test/RandomWalk 1000     # one model, every configuration
./check-all.sh                              # every ported model, about 10 minutes
```

The first run fetches LSD's `src/` and `Example/` folders into `target/lsd-src` (72 MB instead of the 2.2 GB
repository). It needs Homebrew GCC (`g++-15`; set `CXX` to use another) and Rust. The status of every model,
rule and macro family is in the [checklist](#checklist) below.

## The reference build (oracle)

[oracle/build-oracle.sh](../oracle/build-oracle.sh) builds LSD's headless engine (`lsdNW`) for one model:

```bash
oracle/build-oracle.sh target/lsd-src Example/SantAnna/K+S-original target/check
```

It pins what would otherwise make LSD's output depend on the machine:

| Setting | Why |
|---|---|
| GCC (`g++-15`), not Apple clang | The random draws come from the C++ standard library; GCC's libstdc++ is what the port copies. Pin the GCC version too, because libstdc++ has changed distribution algorithms between releases |
| `-ffp-contract=off`, no `-march=native` | Stops the compiler fusing multiply-adds depending on the CPU. Rust never fuses |
| `SIG_DIG` 10 → 17 in a copy of `src/decl.h` | 17 significant digits round-trip a double exactly; LSD's default 10 hides small differences |

Run the oracle and the port **on the same machine**: both call the OS math library for `exp`, `log`, `pow` and
friends, and those can differ between operating systems.

Run the oracle with `-c 1` (one thread). Variables flagged for parallel update (K+S has some) make a multi-threaded
LSD's trace order depend on thread timing, although its values and CSV don't change.

Before porting a model, run its oracle twice and confirm the outputs are byte-identical. Done for RandomWalk and
K+S-original. A model that differs from itself (reading uninitialized memory, ordering by address) has to be fixed
or excluded first.

## Exact tier

1. **Trace patch.** LSD's own computation log is GUI-only and prints 6 digits (`src/variab.cpp:509`).
   `oracle/build-oracle.sh` patches the reference's `variable::cal` and `object::write` so that, with
   `LSD_TRACE=file`, every computed value and every write is logged in order as `t, kind, label, value (%.17g)`.
2. **Same trace from the port.** Compat mode logs the same line at the same point.
3. **Compare.** Because compat mode must also reproduce LSD's *order* of computation, the two traces must be
   identical line by line. No object IDs are needed: the first differing line names the step, the equation and its
   object, and the 20 lines before it show what led there.
4. **Loop.** An agent ports, runs both, reads the first divergence, fixes, repeats, until the traces are identical
   for every step of every shipped config of the model. Then compare the CSV outputs as a final check.

`lsd compare` in the CLI is this diff.

## Random numbers to port

Each distribution gets a unit test against draws from a tiny C++ program built with the pinned GCC.

| Distribution | Used by shipped examples | libstdc++ algorithm |
|---|---|---|
| Uniform | 63 calls | `generate_canonical`: two 32-bit draws per double — **done** |
| Beta | 49 | Two gamma draws (LSD's own code) |
| Normal | 38 | Marsaglia polar method; LSD makes a new distribution per draw, so the cached second value is always discarded |
| Uniform integer | 26 | Version-dependent; pin GCC |
| Bernoulli | 16 | Uniform draw compared with p |
| Poisson | 14 | Different methods for small and large means |
| Gamma | via beta | Marsaglia–Tsang, with its own internal normal |
| Lognormal, exponential, others | rare | Port when a model needs them |

Also port LSD's other generators only if a model selects them (`RND_GENERATOR`). Remember the internal `minstd_rand`
(`lc1`) used for random update periods and `uniform_int` inside LSD.

## Statistical tier

Use it only for changes that are meant to alter results: native-mode random numbers, per-object streams, and later
any change to update order.

- Run 50 or more Monte Carlo runs per shipped config, both versions.
- Compare with the model's own analysis scripts. K+S-original ships them in `R/`: aggregates, sector statistics,
  time plots, box plots, elementary-effects and Kriging–Sobol sensitivity analysis.
- Agree the criteria before looking at results: means and standard deviations of key series within Monte Carlo
  error, matching autocorrelations, and the stylized facts the model's papers report.

## Order of models

1. `Example/Test`: RandomWalk (done), LogisticChaos, GameOfLife, Worm, BugsLife, ReplDynMacro, FittingFunction,
   Nets (networks).
2. Small `Example/Literature` models: NelsonWinter, Schelling, LotkaVolterra, Arthur.
3. **K+S-original** (6k lines): the real test, using extension structs, hooks, writes and object entry and exit.
4. The other K+S variants (finance, var_firms, labor, energy, DSK-refresh), then the UFRJ and Market models.

## Pitfalls to watch

- **Containers ordered by memory address** (`set<object*>`, `map<object*,…>`): the port must reproduce whatever
  order the oracle actually produced, or the model must be changed in both.
- **Summation order:** sums and averages over instances must run in LSD's instance order.
- **Casts:** C++ `(int)` of NaN or an out-of-range value is undefined behaviour; Rust's `as` saturates and maps NaN
  to 0. Apple Silicon happens to behave like Rust; x86 does not. Flag such casts during the port.
- **Missing values:** LSD's `USE_NAN` mode lets NaN propagate; match it.
- **Threading:** never compare against an oracle run with within-step threading if any variable is flagged parallel.

## Checklist

Each LSD behaviour stays open until a model that uses it matches the reference LSD exactly (computation trace and
CSV identical). `check-all.sh` re-proves everything in about 10 minutes. Status as of 11 October 2026.

**Exact** means verified against the reference LSD in the model named. **Implemented** means coded from LSD's source
but not yet exercised by a model. **Open** means not implemented.

### Models

| Model | Configs | Computations compared | Result |
|---|---|---|---|
| RandomWalk | 6 | 20.1 million (full length) | Exact, CSV identical |
| LogisticChaos | 3 | 11.0 million (full length) | Exact, CSV identical |
| ReplDynMacro | 2 | 128,000 (full length) | Exact, CSV identical |
| Worm | 2 | 20.7 million (full length) | Exact (no CSV saved) |
| BugsLife | 1 | 13.3 million (full length) | Exact, CSV identical |
| GameOfLife | 5 | 43.1 million (first 100 steps) | Exact, CSV identical |
| Nets | 2 | — | Not comparable: the C++ example returns an uninitialised value (`NodeLinks`); exact with that line fixed (NetsFixed) |
| NetsFixed, NetsGen, NetsOps (`tests/lsd-models/`) | 22 | 236,000 | Exact, CSV identical, network files byte-identical |
| MacroStats, MacroSort, MacroSearch, MacroSearchFast, MacroObjects, MacroHooks (`tests/lsd-models/`) | 6 | about 250,000 | Exact, CSV identical |
| FittingFunction | — | — | Not ported: an LSD 2.0 equation file that LSD 8.1 doesn't build |
| K+S-original | 4 | 90.1 million (full length, 600 steps); also 28 sweep and 60 random parameter configs | Exact, CSV identical |

The `tests/lsd-models/` models were written for this purpose: small LSD models in C++ that call every macro of a
family with randomness, ties, conditions and lags, plus their Rust ports.

### The evaluation rules (ASSESSMENT.md)

| # | Rule | Status |
|---|---|---|
| 1 | Step order: own variables in order, then sons, instance by instance | Exact (all) |
| 2 | On-demand computation, cached once per step | Exact (all); the deadlock error is implemented, untested |
| 3 | The lag rule (`VL(X,1)` is always t−1) | Exact (all) |
| 4 | Lags beyond the buffer read from saved series, only after `USE_SAVED` | Exact (MacroStats) |
| 5 | Parameters; Functions recomputed per call; LSD's `FUNCTION` macro | Exact (BugsLife, GameOfLife) |
| 6 | Periodic variables and their random delays (`<upd: …>`) | Implemented |
| 7 | Adding objects mid-step: copies, blueprint, n at a time, with a given last update | Exact (BugsLife, GameOfLife, MacroObjects, NetsOps) |
| 7 | Deleting objects, also inside `CYCLE_SAFE`, also all instances (`USE_ZERO_INSTANCE`); their saved series in the CSV | Exact (BugsLife, MacroObjects) |
| 7 | Deletion postponed while the object is being computed; deletion stopping the step's traversal | Implemented |
| 8 | Writes (`WRITE`, `WRITES`, `WRITEL(S)`, `WRITELL(S)`, `INCR`, `MULT`) | Exact (BugsLife, GameOfLife, Worm, macro models) |
| 9 | Hooks (`HOOK`, `SHOOK`, `WRITE_HOOK`, `ADDHOOK`, `COUNT_HOOK`) | Exact (MacroHooks) |
| 10 | C++ extension objects (`EXT`) | Exact as typed Rust data (MacroHooks); each model's C++ classes are ported by hand |
| 11 | Within-step threads | Out of scope: compat mode is single-threaded (results are identical without them) |
| — | `UPDATE`, `RECALC`, `LAST_CALC`, `ABORT` | Exact (MacroObjects) |
| — | Networks: 11 generators, Pajek load/save/snapshot, node and link operations, statistics | Exact (NetsGen, NetsOps, NetsFixed) |

### Random numbers

All ten functions (`ran1`, `uniform`, `uniform_int`, `rnd_int`, `norm`, `lnorm`, `gamma`, `beta`, `poisson`,
`bernoulli`) are bit-identical to GCC 15's libstdc++ on 1,000,000 interleaved draws over five seeds
(`tests/rng_exact.rs`).

### Macros

The shipped examples use 183 macros in 115 families.

- **Exact:** the equation core (`EQUATION`, `RESULT`, `END_EQUATION`, `FUNCTION`, `EQUATION_DUMMY`, `PARAMETER`, `CURRENT`, `T`).
- **Exact:** values and writes (`V`, `VL`, `VS`, `VLS`, `V_CHEAT`, `WRITE*`, `INCR*`, `MULT*`).
- **Exact:** cycles and searches (`CYCLE*`, `SEARCH*`, `SEARCH_CND*`, `SEARCH_INST`, turbo search).
- **Exact:** all statistics in their `L`/`S`/`_CND` forms (`SUM`, `MAX`, `MIN`, `AVE`, `WHTAVE`, `SD`, `MED`, `PERC`, `COUNT`, `COUNT_ALL`, `STAT`, `MAVE`, `WHTMAVE`).
- **Exact:** sorting (`SORT`, `SORT2`) and random draws of objects (`RNDDRAW*`).
- **Exact:** objects (`ADDOBJ*`, `ADDNOBJ*`, `DELETE`), hooks, extensions and navigation.
- **Exact:** run control (`RECALC`, `LAST_CALC`, `UPDATE*`, `ABORT`, `USE_NAN`, `USE_SAVED`, `NO_SEARCH`, `USE_ZERO_INSTANCE`).
- **Exact:** the whole network family and the random-number macros.
- **Open:** `INTERACT` (interactive debugger only), sorting objects by network node id, and lattice graphics (headless no-ops).

### Harness lessons

- LSD's no-window makefile hard-codes `g++`, which on macOS is Apple clang. `oracle/build-oracle.sh` forces GCC with
  `CC_NW`; check with `otool -L lsdNW` that it links `libstdc++`.
- LSD 8.1 refuses `SEED 0`, which 27 example configs contain; `check.sh` runs them with seed 1 on both sides.
- `SORT` ties depend on the C library's `qsort`. Exactness here is against macOS libc; on Linux, glibc orders ties
  differently, so a Linux reference would need glibc's `qsort` ported instead.
- Example models can contain bugs that make the original non-deterministic (Nets reads uninitialised memory). Run the
  reference twice before porting; `check.sh`'s first divergence shows it immediately.
- The CSV names of deleted objects come from LSD's per-step cached instance positions, not their current position.
