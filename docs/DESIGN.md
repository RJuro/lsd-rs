# Design for a Rust rewrite

*The design notes written before the rewrite. The engine follows them in compat mode. The command line ended up a
little different from the sketch below (`explain` instead of `trace`, plus `new`, `series` and `compare`); the
current tools are in [AGENTS.md](../AGENTS.md). Native mode, the GUI ideas and Parquet output are not built.*

## Rust or Python?

LSD's core idea isn't tied to a language: variables are equations, computed only when asked for, cached once per
step, with lags built in and no manual update order. That takes about 200 lines of Python. So the real choice is
which language the equations run in.

| | Python | Rust |
|---|---|---|
| Speed of agent loops | Typically 10–100× slower; interacting heterogeneous agents don't vectorize, and numba/JAX handle changing populations poorly | Fast |
| Catching errors | At runtime, maybe at step 400 of run 37 | At compile time: misspelled names, wrong types, missing cases. `cargo` reports errors as JSON, so agents can fix them in a loop |
| Readable by co-authors, referees, students | Yes | Mostly no |
| Edit → run loop | Instant; inspect state in a notebook | A few seconds of recompiling |
| Analysis and estimation | In process: Polars, statsmodels, SALib, PyMC | Write files, analyse in Python or R |
| Browser builds for teaching | Pyodide or marimo's WASM mode; heavy and slow | Small, fast WASM |
| Agents writing it | Equally good | Equally good |

Speed decides it. A single K+S run takes 13 s in LSD; that doesn't matter. Calibration and sensitivity analysis need
10⁴–10⁵ runs, and then it's hours versus weeks.

**Recommendation:** equations and engine in Rust, everything around them (setup, sweeps, analysis, notebooks) in
Python, like Polars (a Rust engine with a Python face). The equations themselves can't be split: calling into Python
for every equation, agent and step loses the speed. Rust also has the stronger case for agents, because the compiler
checks their work before anything runs.

Existing Rust agent-based frameworks don't fit: [krABMaga](https://www.jasss.org/27/2/4.html) (JASSS 2024) schedules
agents in the MASON style and has no on-demand equations.

## Two modes

- **Compat mode** reproduces LSD exactly: the same evaluation order, immediate object creation and deletion, and
  random draws matching GCC's standard library. Its job is to validate every model port bit for bit
  (see [VALIDATION.md](VALIDATION.md)). Single-threaded.
- **Native mode** turns on improvements that change results: platform-independent random numbers and per-object
  random streams. Opt-in per model, validated statistically against compat mode.

Keep compat mode permanently. It is the reference for every future model port.

## Engine

- **Objects** live in one store and are addressed by IDs that carry a generation number. The tree is a parent ID plus
  per-type child lists in LSD's instance order. A deleted object's ID becomes invalid immediately, so a stale hook
  is a clear error instead of a dangling pointer.
- **Values** are stored per variable: a slot per object that is empty, computing, or done; a lag buffer sized to the
  longest declared lag; and the saved series, which LSD also uses for longer lags.
- **Ownership.** Equations get read access to the model, and the values, the tree and extension data are written
  through interior mutability (`Cell`/`RefCell`). This avoids fighting Rust's borrow checker while still allowing
  LSD's immediate structural changes. Parallelism happens across runs, not inside one, so no locks are needed.
- **Loops.** Asking for a value that is still `computing` is an error that prints the whole chain of variables,
  replacing LSD's "deadlock" message.
- **Hooks** are optional object IDs. **Extension data** (LSD's `EXT`, used heavily in K+S) becomes a typed Rust struct
  per object type, holding vectors and ordered maps of IDs instead of pointers. Never order anything by memory address.
- **Random numbers.** Compat: port GCC libstdc++'s algorithms (Mersenne Twister, uniform, uniform integer, normal,
  gamma, Poisson, Bernoulli, …), pinned to one GCC version. Native: own implementations, plus counter-based streams
  keyed by (seed, step, variable, object). With keyed streams, adding an unrelated variable no longer shifts every
  other draw, and parallel runs stay reproducible.
- **Parallelism** across Monte Carlo runs and sweep points. Skip LSD-style threading within a step: on K+S-original it
  was 1.4–3× slower than one thread, with identical results.
- **Output.** Parquet plus a `manifest.json` per run (engine version, model commit, seed, parameter and config hashes).
  Keep CSV and `.res`/`.tot` export so LSD's R packages (LSDinterface, LSDsensitivity, LSDirf) keep working.

## Writing models

- **Structure and initial values:** keep the `.lsd` file format, reading and writing it unchanged. It's plain text,
  diffs well, agents can edit it, and old LSD can still open the configs. Replace it only if it starts to hurt.
- **Equations:** Rust. A build step generates typed names from the `.lsd` structure, so a misspelled variable is a
  compile error:

```cpp
// LSD
EQUATION("Price")
RESULT((1 + V("markup")) * V("UnitCost"))
```

```rust
// rewrite
fn price(c: &Ctx<Firm>) -> f64 {
    (1.0 + c.p(Firm::markup)) * c.v(Firm::UnitCost)
}
```

| LSD | Rewrite |
|---|---|
| `V("X")`, `VL("X",1)` | `c.v(T::X)`, `c.lag(T::X, 1)` |
| `VS(obj,"X")` | `c.on(obj).v(T::X)` |
| `SUM("X")`, `AVE`, `MAX`, `STAT`, … | `c.sum(Child::X)` and friends, over children in LSD order |
| `CYCLE(cur,"Firm")` | `for f in c.children::<Firm>()` |
| `WRITE`, `INCR`, `MULT` | `c.write(obj, T::X, v)`, … |
| `ADDOBJ`, `DELETE` | `c.add::<Firm>()`, `c.delete(obj)` |
| `HOOK`, `SHOOK` | `c.hook(…)` returning an object ID |
| `EXT` | A typed field on the object's extension struct |
| `RND`, `UNIFORM`, `norm`, `beta`, … | `c.rng().uniform(a, b)`, … |

The ~40 operations behind LSD's 340 macros become ~40 methods; the variants become arguments.

Agents port each model's `fun_*.cpp` to Rust, checked by the exact-match harness.

## CLI for agents

Rules: no prompts, JSON on stdout, text messages on stderr, meaningful exit codes, short summaries by default, and
every run reproducible from its manifest.

| Command | What it does |
|---|---|
| `lsd describe` | Objects, variables, parameters, lags, and which equation reads what |
| `lsd check` | Structure, dependency loops and missing initial values, reported as JSON |
| `lsd run --seed 1 --steps 600 --mc 50 --set Firm.markup=0.2` | Parquet plus manifest; prints summary statistics |
| `lsd trace 'Firm[3].Price@120'` | The tree of values behind one result; replaces LSD's GUI debugger |
| `lsd sweep design.toml` | Latin hypercube, NOLH or Sobol designs into one results table |
| `lsd compare oracle.trace port.trace` | First divergence between two runs (see [VALIDATION.md](VALIDATION.md)) |

Skip a `query` command: DuckDB, Polars and R's `arrow` already read Parquet. Skip an MCP server until some agent
can't call a CLI. Ship an `AGENTS.md` explaining the evaluation rules from [ASSESSMENT.md](ASSESSMENT.md); it is the
cheapest and most useful piece.

## GUI in 2026

What each part of LSD's GUI becomes:

| LSD today | Rewrite |
|---|---|
| Model browser | `lsd describe` and an automatically drawn dependency graph (lag links dashed), the view LSD most lacks |
| Initial values editor | The `.lsd` text file, edited by you or an agent; `--set` for one-off changes |
| Sensitivity setup (`set_all.cpp`, NOLH tables) | `lsd sweep` |
| Analysis of results (`analysis.cpp`, gnuplot) | Python package returning Polars tables in marimo or Jupyter; R reads Parquet |
| Run-time plots | `lsd run --view` streams to [Rerun](https://rerun.io), a Rust viewer with a scrubbable timeline; no plotting code to write. Built for robotics, so replace it if cross-section plots get awkward |
| Debugger (`debug.cpp`) | `lsd trace` |
| Model report (`report.cpp`) | Generated from `lsd describe` |
| Model manager / editor (LMM) | Your editor and a coding agent |
| Lattice and network windows | Rerun, or the teaching page |
| — | **Teaching page:** engine and model compiled to WASM, with sliders and live plots; students run K+S without installing anything |

Skip a structure editor: the model is a text file.

## What gets better

- Names checked at compile time instead of string lookups that fail mid-run.
- IDs instead of raw pointers: no dangling pointers after `DELETE`.
- The same results on every OS, and reproducible parallel runs (native mode).
- All cores used for Monte Carlo runs and sweeps; no within-step threading overhead.
- Parquet output with manifests; old formats still exported.
- A CLI and `AGENTS.md` that let coding agents build, run, trace and validate models without a GUI.
- An automated exact-match test suite, which LSD doesn't have today.
