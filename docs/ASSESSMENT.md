# Assessment of the LSD repository

*Written before the rewrite, to scope it. What was built is described in the [README](../README.md) and
[VALIDATION.md](VALIDATION.md).*

Repository: [marcov64/Lsd](https://github.com/marcov64/Lsd), master at commit `2aa874c` (6 May 2026).
Measurements: Apple M1 Pro (8 cores), GCC 15.2 (Homebrew), Rust 1.93, 10 October 2026.
Line references point to that commit, e.g. `src/variab.cpp:271`.

## Status

- LSD 8.1, dated July 2023 (`src/common.h:31`). GPL.
- About 100 commits since August 2022: 96 by Marcelo Pereira (two accounts), 4 by Marco Valente.
  The latest commit on master fixes a typo in a slide.
- No automated tests and no CI.

## Size

The repository is 2.2 GB. Almost none of it is LSD itself:

| Folder | Size | What it is |
|---|---|---|
| `gnu/` | 709 MB, 13k files | Bundled Windows GCC toolchain |
| `installer/` | 48 MB | Installers |
| `Example/` | 25 MB | 40 example models |
| `src/` | 22 MB | Source, plus a Mac app bundle (9 MB), Eigen (6 MB), Tk themes (3 MB) |
| `Manual/` | 15 MB | Documentation |

The source itself is about 80k lines:

| Part | Files | Lines |
|---|---|---|
| Headless engine (what `lsdNW` links) | `object.cpp` 3,759 · `util.cpp` 2,649 · `lsdmain.cpp` 2,331 · `common.cpp` 2,076 · `file.cpp` 1,800 · `nets.cpp` 1,627 · `variab.cpp` 1,116 | 15,358 |
| Engine headers | `decl.h`, `common.h`, `fun_head.h` (the macros), `check.h` | 2,285 |
| GUI and tools, C++ | `analysis.cpp` 10,167 · `interf.cpp` 8,139 · `modman.cpp` (model manager) 5,801 · `debug.cpp` 2,293 · `report.cpp` 2,256 · `set_all.cpp` 1,989 · others | ~35,000 |
| GUI, Tcl | `tkdiff.tcl` 9,765 and `tkcon.tcl` 6,386 (both third-party), plus LSD's own windows | ~26,000 |
| Example models | 40 models in `Example/` | 89,000 |

A rewrite ports the first two rows. The GUI rows get replaced, not ported (see [DESIGN.md](DESIGN.md)).

## The equation language

- 340 macros (`macro_keywords.txt`). They are about 40 operations, each in variants:
  current value (`V`), lagged (`VL`), on another object (`VS`), both (`VLS`), and conditional (`_CND`) forms.
- The shipped examples use 182 of the 340. Unused families include `MED*`, `MAVE*`, `PERC*` and most `STAT*`/`COUNT_ALL*` variants.
- Every lookup is by name at runtime. `V("Price")` expands to `p->cal( p, "Price", 0 )` (`src/fun_head.h:332`),
  which searches the object tree for a variable with that label. A misspelled name is a runtime error, possibly deep into a run.
- Equations are C++ lambdas stored in a map keyed by variable name (`src/fun_head.h:210`).

## How LSD evaluates a model

These are the rules a port must reproduce to match LSD exactly.

1. **Step order.** Each step calls `update` on the root object. Each object computes its own variables in
   declaration order, then recurses into its children, by type and then by instance order (`src/object.cpp:390`).
2. **On demand.** `variable::cal` (`src/variab.cpp:271`): a variable not yet computed this step shifts its lag buffer,
   runs its equation, and caches the result. Asking for a variable that is currently being computed is a "deadlock" error.
3. **The lag rule.** The effective lag is `lag - 1` if the variable hasn't been computed yet this step, otherwise `lag`.
   So `VL("X",1)` always means X at t−1, whichever order things ran in (`src/variab.cpp:304`).
4. **Long lags.** Lags beyond the declared buffer are read from the saved series, if the variable is saved.
5. **Parameters and functions.** Parameters return their stored value. "Functions" are recomputed on every call
   from an equation and never cached.
6. **Periodic variables.** Computed every `period` steps. The optional random extra delay is drawn from a separate
   internal generator (`rnd_int`, which uses `lc1`), so these draws don't touch the main random stream.
7. **Structural changes are immediate.** `ADDOBJ` creates objects mid-step. `DELETE` frees the object at once,
   unless it is being computed; then deletion is postponed (`wait_delete`). Deleting an object while it updates
   stops the rest of its update (`src/object.cpp:1740`).
8. **Writes.** Equations can overwrite other variables (`WRITE*`, `INCR`, `MULT`).
9. **Hooks.** Objects store pointers to other objects (`HOOK` static, `SHOOK` dynamic).
10. **Extensions.** Any C++ object can be attached to an LSD object (`ADDEXT`, `V_EXT`, `EXEC_EXT`).
11. **Within-step threads.** Variables flagged as parallel are computed by worker threads, with a mutex per
    variable and per random generator.

## Random numbers

- Generators are from the C++ standard library: two `minstd_rand` (internal `lc1`, user `lc2`), `mt19937`,
  `mt19937_64`, `ranlux24`, `ranlux48` and `random_device` (`src/util.cpp:2091`). The default is `mt19937`
  (`ran_gen_id = 2`, `src/util.cpp:2077`), seeded from the config's `SEED`.
- Every draw builds a fresh standard-library distribution object (`src/util.cpp:2247`–`2560`): uniform, uniform
  integer, normal, lognormal, gamma, Bernoulli, Poisson, geometric, binomial, Cauchy, chi-squared, exponential, F, t and Weibull.
  Beta is two gamma draws.
- **So the random draws depend on which C++ standard library compiled the model.** GCC's libstdc++ and Apple clang's
  libc++ implement several of these distributions differently. LSD's Mac setup uses Apple's command line tools
  (clang and libc++); the Windows install bundles GCC 12.2. The same model and seed can give different results by OS.
- The default flags `-march=native -O3` (`src/system_options-*.txt`) also let the compiler fuse multiply-adds on CPUs
  that support it, so results can depend on the machine.
- Distributions used by the shipped examples: uniform (63 calls), beta (49), normal (38), uniform integer (26),
  Bernoulli (16), Poisson (14), plus a few CDFs.

## The K+S models

| | K+S-original | K+S-labor |
|---|---|---|
| Lines of model code | 5,979 | 10,796 |
| Equations | 261 | 445 |
| `V`/`VL`/`VS`/`VLS` calls | 1,050 | 1,967 |
| `WRITE`/`INCR`/`MULT` | 154 | 318 |
| Sums, averages, stats | 118 | 204 |
| Hook macros | 33 | 144 |
| Extension macros | 48 | 93 |
| `CYCLE` loops | 38 | 102 |
| `ADDOBJ` / `DELETE` | 8 / 10 | 15 / 20 |

The extension objects are ordinary C++ (`fun_KS_class.h`): structs holding STL lists, maps, sets and vectors of
raw object pointers, for job-application queues, wage offers, vintage tables and firm-ID maps. This is program logic,
not equations, and is the bulk of the porting work.

One `set<object*>` is ordered by memory address (`fun_KS_worker.h:112`). In that spot the order doesn't change
results, but address-ordered containers are a reproducibility hazard to check for in every model.

## Measured on this Mac

| What | Result |
|---|---|
| Build the headless engine plus RandomWalk | 2.4 s |
| RandomWalk, two runs, same seed | Byte-identical output |
| K+S-original, Benchmark config, 600 steps, `-c 1` (one thread), no output files | 12.9 s |
| Same with default threading (8 cores) | 17.8 s and 30.0 s on two runs, with 17–21 s of system time |
| Same with CSV output: one thread / default threading | 14.7 s / 43.4 s |
| Results, one thread vs default threading | Byte-identical |
| Saved series in K+S-original Benchmark | 19,911 columns × 600 steps |
| Rust copy of RandomWalk vs LSD | 1,000 of 1,000 steps bit-identical |

Correction (11 October): these reference builds were compiled by Apple clang, not GCC. LSD's no-window makefile
hard-codes `g++`, which on macOS is clang, and overrode the compiler we set. Found while porting GameOfLife, whose
integer draws differ between the two C++ libraries; `oracle/build-oracle.sh` now forces GCC. The RandomWalk match is
unaffected (uniform draws are identical in both libraries); the timings are clang's.

Threading doesn't help this model and costs 1.4–3× the time. Running one LSD process per core for Monte Carlo
repetitions is the better use of the cores.

At 13 s per run, 50 Monte Carlo runs take 11 core-minutes. A Sobol sensitivity analysis with 100,000 runs takes
about 15 core-days, so speed matters for calibration and sensitivity analysis, not for single runs.
