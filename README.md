# lsd-rs

The engine of [LSD](https://github.com/marcov64/Lsd) (Laboratory for Simulation Development) in Rust: it runs LSD
agent-based models headless, bit for bit identical to the original, with a JSON command line and an MCP server so
that coding agents can build, run and inspect models directly.

LSD is the C++ simulation tool behind a large family of evolutionary-economics models, including the
"Keynes meets Schumpeter" (K+S) models. lsd-rs reads LSD's own `.lsd` configuration files. Models are written as
one Rust function per LSD equation, with one method per LSD macro (`c.v("Price")` for `V("Price")`).

## What "identical" means

Every ported model is checked against the original LSD, compiled from its sources, computation by computation:
- **The trace:** every value LSD computes and every value it writes is logged in order, and the Rust engine must log
  exactly the same lines.
- **The output:** the CSV results must be byte for byte the same.

| Models | Configurations | Computations compared |
|---|---|---|
| LSD's `Example/Test` models: RandomWalk, LogisticChaos, ReplDynMacro, Worm, BugsLife, GameOfLife | 19 | 108 million |
| K+S-original (Dosi et al.; LSD version by M. C. Pereira), full length | 4 | 90 million |
| Test models written to cover every network operation and macro family (`tests/lsd-models/`) | 28 | 0.5 million |

All 51 configurations are identical. That includes the random numbers: GCC's `libstdc++` algorithms are reproduced
exactly (one million test draws).

The rules, macros and models that are proven exact, and what is still open, are in
[docs/VALIDATION.md](docs/VALIDATION.md#checklist).

## Install

```bash
cargo install --git https://github.com/RJuro/lsd-rs
lsd-rs models
```

The binary is `lsd-rs`, so it doesn't clash with the `lsd` file lister.

If your git rewrites GitHub URLs to SSH and `cargo install` fails with "no authentication methods succeeded",
run it with `CARGO_NET_GIT_FETCH_WITH_CLI=true`.

## Use it from an agent

```bash
claude mcp add lsd -- lsd-rs mcp      # Claude Code; any MCP client can run `lsd-rs mcp`
```

Then ask the agent, for example: *"Describe examples/K+S-original/Benchmark.lsd, run it for 20 seeds, and show me
what drives unemployment at step 300."* The tools:

| Tool | What it does |
|---|---|
| `lsd_describe` | Objects, instance counts, variables, parameters, lags, initial values of a `.lsd` configuration |
| `lsd_run` | One or many runs (seeds in parallel), parameter overrides, a bounded summary, CSV results |
| `lsd_series` | Read columns of a results file |
| `lsd_explain` | The tree of values behind one variable at one step, the replacement for LSD's debugger |
| `lsd_new`, `lsd_build`, `lsd_check` | Start a new model from a `.lsd` file (one equation stub per variable), build it, check it |
| `lsd_compare` | Check a model against the original LSD |

[AGENTS.md](AGENTS.md) is the guide for agents: the tools, the equation API, and how to port LSD C++ equations.

## Use it yourself

```bash
lsd-rs run K+S-original examples/K+S-original/Benchmark.lsd --runs 20 --set mu1=0.1
lsd-rs explain ReplDynMacro examples/ReplDynMacro/RDSimple.lsd --var Num --t 3 --instance 1_2
lsd-rs new MyModel --from my.lsd          # then write src/equations.rs and `cargo build --release`
```

Every command prints one JSON document; `lsd-rs help` lists them. [examples/](examples/) has the LSD configurations
of the built-in models.

## Speed

| One run on an M1 Pro | LSD 8.1 (one thread) | lsd-rs |
|---|---|---|
| K+S-original, Benchmark, 600 steps | 13.8 s | 4.1 s |
| RandomWalk, largest config (10,000 objects, 1,000 steps), with CSV | 10.5 s | 3.8 s |
| GameOfLife, largest grid, 100 steps | 8.1 s | 8.8 s |

Monte Carlo runs use all cores (`--runs N --jobs J`). LSD's own multi-threading slows K+S down (18.5 s with 8
threads), so lsd-rs doesn't use threads within a run.

## Checking against the original yourself

```bash
git clone https://github.com/RJuro/lsd-rs && cd lsd-rs
./check.sh Example/Test/RandomWalk 1000      # one model, every configuration
./check-all.sh                               # everything, about 10 minutes
```

This needs Homebrew GCC 15 (`brew install gcc`). The first run fetches LSD's sources (72 MB) into `target/lsd-src`
and builds LSD's headless engine with pinned flags ([oracle/build-oracle.sh](oracle/build-oracle.sh)). Exactness
is against LSD built with GCC on macOS: LSD's random numbers depend on the C++ library, and its sort order of
ties on the C library.

## Layout

| Path | What it is |
|---|---|
| [src/model.rs](src/model.rs) | The object tree, LSD's evaluation and update rules, writes, object creation and deletion, saved series, the core equation API |
| [src/macros.rs](src/macros.rs), [src/nets.rs](src/nets.rs) | The other macros (statistics, sorting, searches, hooks, extensions) and networks |
| [src/rng.rs](src/rng.rs), [src/config.rs](src/config.rs), [src/output.rs](src/output.rs) | Random numbers, the `.lsd` reader, LSD's CSV output |
| [src/cli.rs](src/cli.rs), [src/mcp.rs](src/mcp.rs) | The command line and the MCP server |
| [src/models/](src/models/) | Ported models |
| [examples/](examples/) | LSD configurations of the built-in models |
| [docs/](docs/) | The [assessment of LSD](docs/ASSESSMENT.md) the rewrite started from, the [design](docs/DESIGN.md), and [validation](docs/VALIDATION.md) |

## Not included

- **LSD's graphical interface.** lsd-rs is headless; LSD's GUI can still edit the same `.lsd` files.
- **Interactive debugging** (`INTERACT`); `lsd_explain` replaces the debugger's view of values.
- **Lattice graphics.** In a headless run they only draw, so they do nothing.
- **FittingFunction** among the examples: it's an LSD 2.0 equation file that LSD 8.1 itself no longer builds.

## Credits and license

LSD is by Marco Valente (Università dell'Aquila) and Marcelo C. Pereira (University of Campinas). The K+S model's
LSD implementation is by Marcelo C. Pereira, after Dosi, Fagiolo and Roventini. lsd-rs is an independent
reimplementation, not affiliated with the LSD project. Its engine and model ports follow LSD's source code, so it is
distributed, like LSD, under the GNU General Public License, version 3 or later (see [LICENSE](LICENSE)).

Built by [@RJuro](https://github.com/RJuro) with Claude Code.
