# Working with lsd-rs (for coding agents)

`lsd-rs` is the engine of LSD 8.1 (Laboratory for Simulation Development) in Rust. It runs LSD models headless and
reproduces the original LSD bit for bit. A model is a `.lsd` configuration (structure and initial values, LSD's own
text format) plus one Rust function per variable.

## Tools

Use the MCP server, or the same commands on the command line. Every command prints one JSON document.

| MCP tool | CLI | What it does |
|---|---|---|
| `lsd_models` | `lsd-rs models` | Models built in (or in a project) |
| `lsd_describe` | `lsd-rs describe CONFIG [--model M]` | Objects, instance counts, variables, parameters, lags, saved series; with a model, missing equations |
| `lsd_check` | `lsd-rs check MODEL CONFIG` | Every variable has an equation, and the first 3 steps run |
| `lsd_run` | `lsd-rs run MODEL CONFIG [--seed S] [--runs N] [--jobs J] [--steps T] [--set L=V]... [--out DIR] [--no-csv]` | Runs (several seeds in parallel), a bounded summary per saved series, CSV files in LSD's format |
| `lsd_series` | `lsd-rs series CSV [--cols A,B] [--from T] [--to T] [--every K]` | Columns of a results file; `Num` matches all instances `Num_1_3` |
| `lsd_explain` | `lsd-rs explain MODEL CONFIG --var X --t T [--instance 1_3]` | The tree of values X's equation read at step T — what LSD's debugger showed |
| `lsd_new` | `lsd-rs new NAME --from CONFIG [--dir D]` | A model project: a Rust crate with one equation stub per variable |
| `lsd_build` | `cargo build --release --manifest-path P/Cargo.toml` | Build a project; compiler errors come back |
| `lsd_compare` | `lsd-rs compare LSD_MODEL_DIR [--steps N]` | Compare with the original LSD computation by computation (needs Homebrew GCC and LSD's sources) |

Give `project` (the folder `lsd_new` made) to any tool to use that project's models instead of the built-in ones.

Install, then register the server with Claude Code (or any MCP client: the command is `lsd-rs mcp`):

```bash
cargo install --git https://github.com/RJuro/lsd-rs
claude mcp add lsd -- lsd-rs mcp
```

## Making a model

1. Write or obtain the `.lsd` configuration (LSD's GUI writes them; the format is plain text, see any file in
   LSD's `Example/` folder).
2. `lsd_new` with a name and the configuration. It writes `src/equations.rs` with a stub for every variable.
3. Write the equations. `lsd_build`, then `lsd_check`, then `lsd_run`.
4. To see why a value is what it is, `lsd_explain`.

Errors come back with the step and the variable, e.g. `step 12: LSD error: deadlock: equation for 'Price' (object
'Firm') requested its own value while computing it`.

## Equations

One function per LSD `EQUATION`, taking the context and returning the value:

```rust
/// Price: mark-up over unit cost
fn eq_price(c: &mut Ctx) -> f64 {
    let markup = c.v("markup");
    let cost = c.v("UnitCost");
    (1.0 + markup) * cost
}
```

How LSD evaluates, in short:
- Each step, LSD walks the object tree from the top. Each object computes its variables in the order of the
  configuration, then its descendants, instance by instance.
- A variable is computed when first requested in a step, then cached for the rest of the step.
- `c.vl("X", 1)` is always X's value at t−1.
- A variable can't request its own current value while it is being computed. That is a "deadlock" error; use
  its lag instead.

| LSD macro | Rust |
|---|---|
| `V("X")`, `VL("X",1)` | `c.v("X")`, `c.vl("X", 1)` |
| `VS(o,"X")`, `VLS(o,"X",1)` | `c.vs(o, "X")`, `c.vls(o, "X", 1)` |
| `V_CHEAT("X", o)` | `c.v_cheat("X", o)` |
| `CURRENT`, `T` | `c.current()`, `c.t()` |
| `p`, `c` (this object, caller) | `c.p`, `c.c` (an `Option`) |
| `PARENT`, `PARAMETER` | `c.parent()`, `c.parameter()` |
| `CYCLE(cur, "Firm") { … }` | `let mut cur = c.first("Firm"); while let Some(o) = cur { …; cur = c.next(o); }` |
| `CYCLE_SAFE` (deleting inside) | same, but read `c.next(o)` before the body |
| `CYCLES(o, cur, "Firm")` | `c.firsts(o, "Firm")` then `c.next` |
| `WRITE("X", v)`, `WRITES(o, "X", v)` | `c.write("X", v)`, `c.writes(o, "X", v)` |
| `WRITEL`, `WRITELS` (as if at time T) | `c.writel("X", v, T)`, `c.writels(o, "X", v, T)` |
| `INCR`, `INCRS`, `MULT`, `MULTS` | `c.incr`, `c.incrs`, `c.mult`, `c.mults` |
| `SEARCH("Firm")`, `SEARCHS(o, …)` | `c.search("Firm")`, `c.searchs(o, "Firm")` |
| `SEARCH_CND("id", 3)`, `SEARCH_CNDS` | `c.search_cnd("id", 3.0)`, `c.search_cnds(o, "id", 3.0)` |
| `RNDDRAW("Firm", "share")`, `RNDDRAW_FAIR` | `c.rnddraw("Firm", "share")`, `c.rnddraw_fair("Firm")` |
| `ADDOBJ`, `ADDOBJ_EX`, `ADDOBJS_EX` | `c.addobj("Firm")`, `c.addobj_ex("Firm", ex)`, `c.addobjs_ex(o, "Firm", ex)` |
| `DELETE(o)` | `c.delete(o)` |
| `RND`, `UNIFORM`, `rnd_integer`, `norm`, `lnorm`, `gamma`, `beta`, `poisson`, `bernoulli` | `c.rnd()`, `c.uniform(a,b)`, `c.uniform_int(a,b)`, `c.norm(m,s)`, … |
| `USE_NAN`, `LOG` | `c.use_nan()`, `c.log("…")` |

The rest of LSD's macros follow one naming scheme (the macro in snake_case; `l` = lagged, `s` = from another
object, `_cnd` = with a condition), documented at the top of [src/macros.rs](src/macros.rs):

| LSD | Rust |
|---|---|
| `SUM`, `MAX`, `MIN`, `AVE`, `WHTAVE`, `SD`, `MED`, `PERC`, `COUNT`, `COUNT_ALL`, `STAT`, `MAVE`, `WHTMAVE` and their `L`/`S`/`_CND` forms | `c.sum("X")`, `c.suml("X", 1)`, `c.sums(o, "X")`, `c.sum_cnd("X", "Cat", "==", 2.0)`, … |
| `SORT`, `SORT2` | `c.sort("Firm", "share", "DOWN")`, … |
| `INIT_TSEARCH`, `TSEARCH`, `SEARCH_INST`, `RNDDRAW_TOT` | `c.init_tsearch(…)`, `c.tsearch(…)`, `c.search_inst(o)`, `c.rnddraw_tot(…)` |
| `ADDNOBJ`, `ADDOBJL`, `RECALC`, `LAST_CALC`, `UPDATE` | `c.addnobj("Firm", n)`, `c.addobjl(…)`, `c.recalc("X")`, `c.last_calc("X")`, `c.update()` |
| `HOOK(i)`, `SHOOK`, `WRITE_HOOK`, `ADDHOOK` | `c.hook(i)`, `c.shook()`, `c.write_hook(i, o)`, `c.addhook(n)` (hooks are `Option<ObjId>`) |
| `ADDEXT`, `EXT`, `V_EXT`, `EXEC_EXT` (C++ objects) | your own Rust type: `c.add_ext::<T>(o, value)`, `c.ext::<T>(o)`, `c.ext_mut::<T>(o)` |
| `GRANDPARENT`, `ROOT`, `NEXT` | `c.grandparent()`, `c.root()`, `c.next(o)` |
| `USE_SAVED`, `NO_SEARCH`, `USE_ZERO_INSTANCE`, `ABORT` | `c.use_saved()`, `c.no_search()`, `c.use_zero_instance()`, `c.abort()` |
| Networks: `INIT_NET`, `LOAD_NET`, `SAVE_NET`, `SNAP_NET`, `STAT_NET`, `RNDDRAW_NODE`, `SEARCH_NODE`, `ADDLINK`, `V_LINK`, `CYCLE_LINK`, … | `c.init_net(…)`, `c.load_net(…)`, `c.stat_net(…)`, `c.rnddraw_node(…)`, `c.add_link(…)`, `c.first_link(o)`/`c.next_link(l)`, … in [src/nets.rs](src/nets.rs) |

A `close_sim` function (run once after the simulation) is registered as `("close_sim", EqDef::Hook(f))`.

`EQUATION_DUMMY("X", "Y")` and LSD's `FUNCTION` macro are registered as `EqDef::Dummy("Y")` and
`EqDef::Function(f)`; plain equations as `EqDef::Fn(f)`.

## Porting LSD C++ equations

To reproduce the original exactly, keep the C++ order of every value request, write and random draw. The order of
computation is what makes runs identical: don't merge, reorder or hoist calls. Keep the same floating-point
operations in the same order. C's `(int)` truncates; C's `round` is `f64::round`. Check with `lsd_compare` on the
folder holding the original C++ model; the Rust model must have the folder's name.
