//! The `lsd` command line, shared by the engine binary and every model project.
//! Every command prints one JSON document on stdout (`{"error": ...}` on failure, exit code 1);
//! outputs are bounded so an agent can read them directly.

use crate::config::{self, Config, Kind, ObjSpec};
use crate::model::{EqDef, Model, Node};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

pub type Registry = fn(&str) -> Option<Vec<(&'static str, EqDef)>>;

pub const USAGE: &str = "\
lsd-rs models
lsd-rs describe CONFIG.lsd [--model M]
lsd-rs check MODEL CONFIG.lsd
lsd-rs run MODEL CONFIG.lsd [--seed S] [--runs N] [--jobs J] [--steps T] [--set LABEL=VALUE]... [--out DIR] [--no-csv] [--trace FILE]
lsd-rs series RESULTS.csv [--cols A,B] [--from T] [--to T] [--every K] [--max-rows N]
lsd-rs explain MODEL CONFIG.lsd --var X --t T [--instance 1_3] [--seed S] [--depth D]
lsd-rs new NAME --from CONFIG.lsd [--dir DIR]
lsd-rs compare LSD_MODEL_DIR [--steps N] [--lsd-repo PATH]
lsd-rs mcp";

/// Entry point for a binary: parse the command line, print the JSON result, exit.
pub fn main_with(get: Registry, names: &'static [&'static str]) {
    std::panic::set_hook(Box::new(|_| {})); // model errors are reported as JSON instead
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("mcp") {
        crate::mcp::serve(get, names);
        return;
    }
    match dispatch(&args, get, names) {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => {
            println!("{}", json!({ "error": e, "usage": if e.starts_with("usage") { USAGE } else { "" } }));
            std::process::exit(1);
        }
    }
}

/// Run one command; used by the binary and by the MCP server.
pub fn dispatch(args: &[String], get: Registry, names: &[&str]) -> Result<Value, String> {
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let rest = if args.is_empty() { &args[..] } else { &args[1..] };
    let pos: Vec<&String> = positional(rest);
    match cmd {
        "models" => Ok(json!({ "models": names })),
        "help" | "--help" | "-h" => Ok(json!({ "usage": USAGE.lines().collect::<Vec<_>>(), "docs": "https://github.com/RJuro/lsd-rs/blob/main/AGENTS.md" })),
        "describe" => describe(arg(&pos, 0, "CONFIG")?, opt(rest, "--model").map(|m| (m, get))),
        "check" => check(arg(&pos, 0, "MODEL")?, arg(&pos, 1, "CONFIG")?, get),
        "run" => run(arg(&pos, 0, "MODEL")?, arg(&pos, 1, "CONFIG")?, rest, get),
        "series" => series(arg(&pos, 0, "RESULTS.csv")?, rest),
        "explain" => explain(arg(&pos, 0, "MODEL")?, arg(&pos, 1, "CONFIG")?, rest, get),
        "new" => new_project(arg(&pos, 0, "NAME")?, rest),
        "compare" => compare(arg(&pos, 0, "LSD_MODEL_DIR")?, rest),
        _ => Err(format!("usage: unknown command '{}'", cmd)),
    }
}

// ------------------------------------------------------------------ argument helpers

const FLAGS_WITH_VALUE: &[&str] = &["--model", "--seed", "--runs", "--jobs", "--steps", "--set", "--out", "--trace",
    "--cols", "--from", "--to", "--every", "--max-rows", "--var", "--t", "--instance", "--depth", "--dir", "--lsd-repo"];

fn positional(rest: &[String]) -> Vec<&String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        if FLAGS_WITH_VALUE.contains(&rest[i].as_str()) {
            i += 2;
        } else if rest[i].starts_with("--") {
            i += 1;
        } else {
            out.push(&rest[i]);
            i += 1;
        }
    }
    out
}

fn arg<'a>(pos: &[&'a String], i: usize, name: &str) -> Result<&'a str, String> {
    pos.get(i).map(|s| s.as_str()).ok_or_else(|| format!("usage: missing {}", name))
}

fn opt<'a>(rest: &'a [String], name: &str) -> Option<&'a str> {
    rest.iter().position(|a| a == name).and_then(|i| rest.get(i + 1)).map(String::as_str)
}

fn opts<'a>(rest: &'a [String], name: &str) -> Vec<&'a str> {
    rest.windows(2).filter(|w| w[0] == name).map(|w| w[1].as_str()).collect()
}

fn num<T: std::str::FromStr>(rest: &[String], name: &str) -> Result<Option<T>, String> {
    opt(rest, name).map(|s| s.parse().map_err(|_| format!("{} expects a number, got '{}'", name, s))).transpose()
}

fn stem_of(path: &str) -> String {
    Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn load(path: &str) -> Result<Config, String> {
    config::load(Path::new(path)).map_err(|e| format!("{}: {}", path, e))
}

fn model_eqs(get: Registry, model: &str) -> Result<Vec<(&'static str, EqDef)>, String> {
    get(model).ok_or_else(|| format!("unknown model '{}'; see `lsd-rs models`", model))
}

fn elems(spec: &ObjSpec, out: &mut Vec<(String, String, Kind)>) {
    for e in &spec.elems {
        out.push((spec.label.clone(), e.label.clone(), e.kind));
    }
    for s in &spec.sons {
        elems(s, out);
    }
}

// ------------------------------------------------------------------ describe / check

fn describe(path: &str, model: Option<(&str, Registry)>) -> Result<Value, String> {
    let cfg = load(path)?;
    fn obj(s: &ObjSpec, parent: Option<&str>) -> Vec<Value> {
        let elements: Vec<Value> = s.elems.iter().map(|e| {
            let firsts: Vec<f64> = e.values.iter().filter_map(|v| v.first().copied()).collect();
            let (lo, hi) = firsts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &x| (a.0.min(x), a.1.max(x)));
            json!({
                "label": e.label,
                "kind": match e.kind { Kind::Var => "variable", Kind::Param => "parameter", Kind::Func => "function" },
                "lags": e.num_lag,
                "saved": e.save,
                "initial": if firsts.is_empty() { Value::Null } else if lo == hi { json!(lo) } else { json!({ "min": lo, "max": hi }) },
            })
        }).collect();
        let mut v = vec![json!({
            "object": s.label,
            "parent": parent,
            "instances": s.total(),
            "computed_each_step": s.to_compute,
            "elements": elements,
        })];
        for son in &s.sons {
            v.extend(obj(son, Some(&s.label)));
        }
        v
    }
    let mut out = json!({
        "config": path,
        "steps": cfg.max_step,
        "seed": cfg.seed,
        "runs": cfg.sim_num,
        "objects": obj(&cfg.root, None),
    });
    if let Some((m, get)) = model {
        out["equations"] = coverage(&cfg, &model_eqs(get, m)?);
    }
    Ok(out)
}

/// Which variables lack an equation, and which equations match no element.
fn coverage(cfg: &Config, eqs: &[(&'static str, EqDef)]) -> Value {
    let mut all = Vec::new();
    elems(&cfg.root, &mut all);
    let missing: Vec<String> = all.iter().filter(|(_, l, k)| *k != Kind::Param && !eqs.iter().any(|(e, _)| e == l))
        .map(|(o, l, _)| format!("{} (in {})", l, o)).collect();
    let unused: Vec<&str> = eqs.iter().filter(|(_, d)| !matches!(d, EqDef::Hook(_))).map(|(e, _)| *e).filter(|e| !all.iter().any(|(_, l, _)| l == e)).collect();
    json!({ "missing": missing, "unused": unused, "complete": missing.is_empty() })
}

fn check(model: &str, path: &str, get: Registry) -> Result<Value, String> {
    let mut cfg = load(path)?;
    let eqs = model_eqs(get, model)?;
    let cov = coverage(&cfg, &eqs);
    cfg.max_step = cfg.max_step.min(3);
    let trial = run_one(&cfg, &eqs, 1, None, &stem_of(path)).map(|_| "ok".to_string()).unwrap_or_else(|e| e);
    Ok(json!({ "model": model, "config": path, "equations": cov, "first_3_steps": trial }))
}

// ------------------------------------------------------------------ run

struct RunOut {
    m: Model,
    secs: f64,
}

fn run_one(cfg: &Config, eqs: &[(&'static str, EqDef)], seed: u32, trace: Option<&str>, name: &str) -> Result<RunOut, String> {
    let mut m = Model::new(cfg, eqs, seed);
    m.sim_name = name.to_string();
    m.nets.sim_name = name.to_string();
    m.nets.seed = Some(seed as i64);
    if let Some(t) = trace {
        m.trace_to(t);
    }
    let t0 = Instant::now();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.run()));
    match r {
        Ok(()) => Ok(RunOut { secs: t0.elapsed().as_secs_f64(), m }),
        Err(p) => {
            let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown error".into());
            Err(format!("step {}: {}", m.t, msg))
        }
    }
}

/// `--set LABEL=VALUE`: the initial value of every instance (all lags of a variable).
fn apply_sets(cfg: &mut Config, sets: &[&str]) -> Result<(), String> {
    fn set(s: &mut ObjSpec, lab: &str, v: f64) -> bool {
        let mut hit = false;
        for e in s.elems.iter_mut().filter(|e| e.label == lab) {
            for x in e.values.iter_mut() {
                if e.kind == Kind::Param { *x = vec![v] } else { x.iter_mut().for_each(|y| *y = v) }
            }
            e.initialized = true;
            hit = true;
        }
        s.sons.iter_mut().fold(hit, |h, son| set(son, lab, v) || h)
    }
    for s in sets {
        let (lab, v) = s.split_once('=').ok_or_else(|| format!("--set expects LABEL=VALUE, got '{}'", s))?;
        let v: f64 = v.parse().map_err(|_| format!("--set {}: '{}' is not a number", lab, v))?;
        if !set(&mut cfg.root, lab, v) {
            return Err(format!("--set: no element '{}' in the configuration", lab));
        }
    }
    Ok(())
}

/// Per saved element: the mean over its instances, as a series; then a short summary of that series.
fn summarize(m: &Model) -> Map<String, Value> {
    let steps = m.max_step;
    let mut acc: Vec<(String, Vec<f64>, Vec<u32>)> = Vec::new();
    let mut add = |label: &str, data: &[f64], start: i32, end: i32| {
        let i = match acc.iter().position(|a| a.0 == label) {
            Some(i) => i,
            None => { acc.push((label.to_string(), vec![0.0; steps as usize], vec![0; steps as usize])); acc.len() - 1 }
        };
        for t in 1..=steps {
            if t >= start && t <= end {
                if let Some(&x) = data.get((t - start) as usize) {
                    if !x.is_nan() {
                        acc[i].1[(t - 1) as usize] += x;
                        acc[i].2[(t - 1) as usize] += 1;
                    }
                }
            }
        }
    };
    for o in 0..m.objs.len() {
        if let Some(obj) = &m.objs[o] {
            for v in obj.vars.iter().filter(|v| v.save && !v.data.is_empty()) {
                add(&v.label, &v.data, v.start, v.end);
            }
        }
    }
    for d in &m.cemetery {
        add(&d.label, &d.data, d.start, d.end);
    }
    let mut out = Map::new();
    for (label, sum, n) in acc {
        let s: Vec<f64> = sum.iter().zip(&n).map(|(a, &k)| if k > 0 { a / k as f64 } else { f64::NAN }).collect();
        let valid: Vec<f64> = s.iter().copied().filter(|x| !x.is_nan()).collect();
        if valid.is_empty() {
            continue;
        }
        let tail = &valid[valid.len() - (valid.len() / 10).max(1)..];
        out.insert(label, json!({
            "final": s.last().copied().filter(|x| !x.is_nan()),
            "instances_final": n.last().copied().unwrap_or(0),
            "mean_last_10pct": tail.iter().sum::<f64>() / tail.len() as f64,
            "min": valid.iter().copied().fold(f64::INFINITY, f64::min),
            "max": valid.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        }));
    }
    out
}

fn run(model: &str, path: &str, rest: &[String], get: Registry) -> Result<Value, String> {
    let eqs = model_eqs(get, model)?;
    let mut cfg = load(path)?;
    apply_sets(&mut cfg, &opts(rest, "--set"))?;
    if let Some(s) = num::<u32>(rest, "--steps")? {
        cfg.max_step = s;
    }
    let first = num::<u32>(rest, "--seed")?.unwrap_or(if cfg.seed == 0 { 1 } else { cfg.seed });
    if first == 0 {
        return Err("seed must be > 0, as in LSD".into());
    }
    let runs = num::<u32>(rest, "--runs")?.unwrap_or(1).max(1);
    let jobs = num::<usize>(rest, "--jobs")?.unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get())).max(1);
    let csv = !rest.iter().any(|a| a == "--no-csv");
    let out_dir = PathBuf::from(opt(rest, "--out").unwrap_or("lsd-out"));
    if csv {
        std::fs::create_dir_all(&out_dir).map_err(|e| format!("{}: {}", out_dir.display(), e))?;
    }
    let trace = opt(rest, "--trace");
    if trace.is_some() && runs > 1 {
        return Err("--trace works with a single run".into());
    }
    let stem = Path::new(path).file_stem().unwrap().to_string_lossy().to_string();
    let next = AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<(u32, Result<(f64, Option<String>, Map<String, Value>), String>)>> = Default::default();
    std::thread::scope(|sc| {
        for _ in 0..jobs.min(runs as usize) {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= runs as usize {
                    break;
                }
                let seed = first + i as u32;
                let r = run_one(&cfg, &eqs, seed, trace, &stem).map(|o| {
                    let file = csv.then(|| {
                        let f = out_dir.join(format!("{}_{}.csv", stem, seed));
                        std::fs::write(&f, crate::output::csv(&o.m)).ok();
                        f.display().to_string()
                    });
                    (o.secs, file, summarize(&o.m))
                });
                results.lock().unwrap().push((seed, r));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| r.0);
    let mut runs_out = Vec::new();
    let mut finals: Map<String, Value> = Map::new();
    let mut ok: Vec<&Map<String, Value>> = Vec::new();
    for (seed, r) in &results {
        match r {
            Ok((secs, file, summary)) => {
                runs_out.push(json!({ "seed": seed, "seconds": (secs * 1000.0).round() / 1000.0, "csv": file }));
                ok.push(summary);
            }
            Err(e) => runs_out.push(json!({ "seed": seed, "error": e })),
        }
    }
    if ok.len() == 1 {
        finals = ok[0].clone();
    } else if ok.len() > 1 {
        // across runs: mean and standard deviation of each element's final instance mean
        for (label, _) in ok[0].iter() {
            let xs: Vec<f64> = ok.iter().filter_map(|s| s.get(label)?.get("final")?.as_f64()).collect();
            if xs.is_empty() {
                continue;
            }
            let mean = xs.iter().sum::<f64>() / xs.len() as f64;
            let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (xs.len().max(2) - 1) as f64).sqrt();
            finals.insert(label.clone(), json!({ "final_mean": mean, "final_sd": sd, "runs": xs.len() }));
        }
    }
    let failed = results.iter().filter(|r| r.1.is_err()).count();
    let runs_json = if runs_out.len() <= 5 { Value::Array(runs_out) } else {
        // many runs: say where the files are and list only the failures
        let secs: f64 = results.iter().filter_map(|r| r.1.as_ref().ok().map(|o| o.0)).sum();
        json!({
            "count": runs_out.len(), "seeds": format!("{}-{}", first, first + runs - 1),
            "csv": csv.then(|| out_dir.join(format!("{}_<seed>.csv", stem)).display().to_string()),
            "seconds_total": (secs * 1000.0).round() / 1000.0,
            "errors": runs_out.iter().filter(|r| r.get("error").is_some()).cloned().collect::<Vec<_>>(),
        })
    };
    Ok(json!({
        "model": model, "config": path, "steps": cfg.max_step, "runs": runs_json,
        "failed": failed, "summary": finals,
        "note": "summary: per saved element, the mean over its instances (single run: final value, last-10% mean, min, max; several runs: mean and sd of the final value across runs)",
    }))
}

// ------------------------------------------------------------------ series

fn series(path: &str, rest: &[String]) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or("empty file")?.split(',').collect();
    let want: Vec<String> = opt(rest, "--cols").map(|c| c.split(',').map(String::from).collect()).unwrap_or_default();
    // a requested name matches the column itself or all its instances ("Num" matches "Num_1_3")
    let cols: Vec<usize> = header.iter().enumerate().filter(|(_, h)| {
        want.is_empty() || want.iter().any(|w| *h == w || h.strip_prefix(w.as_str()).map_or(false, |r| r.starts_with('_')))
    }).map(|(i, _)| i).take(50).collect();
    let (from, to) = (num::<usize>(rest, "--from")?.unwrap_or(1), num::<usize>(rest, "--to")?.unwrap_or(usize::MAX));
    let every = num::<usize>(rest, "--every")?.unwrap_or(1).max(1);
    let max_rows = num::<usize>(rest, "--max-rows")?.unwrap_or(500);
    let rows: Vec<(usize, Vec<&str>)> = lines.enumerate().map(|(i, l)| (i + 1, l.split(',').collect::<Vec<_>>()))
        .filter(|(t, _)| *t >= from && *t <= to && (t - from) % every == 0).collect();
    let truncated = rows.len() > max_rows;
    let mut data = Map::new();
    for &c in &cols {
        let v: Vec<Value> = rows.iter().take(max_rows).map(|(_, r)| r.get(c).and_then(|x| x.parse::<f64>().ok()).map_or(Value::Null, |x| json!(x))).collect();
        data.insert(header[c].to_string(), Value::Array(v));
    }
    Ok(json!({
        "file": path, "columns_total": header.len(), "columns_shown": cols.len(),
        "t": rows.iter().take(max_rows).map(|r| r.0).collect::<Vec<_>>(),
        "truncated": truncated, "data": data,
    }))
}

// ------------------------------------------------------------------ explain

fn node_json(n: &Node, depth: usize) -> Value {
    let mut v = json!({ "var": n.label, "object": n.object, "value": n.value });
    if n.lag != 0 {
        v["lag"] = json!(n.lag);
    }
    if n.computed {
        v["computed_now"] = json!(true);
    }
    if !n.children.is_empty() {
        v["reads"] = if depth == 0 { json!(format!("{} more", n.children.len())) } else {
            Value::Array(n.children.iter().map(|c| node_json(c, depth - 1)).collect())
        };
    }
    v
}

fn explain(model: &str, path: &str, rest: &[String], get: Registry) -> Result<Value, String> {
    let eqs = model_eqs(get, model)?;
    let mut cfg = load(path)?;
    let var = opt(rest, "--var").ok_or("usage: explain needs --var")?;
    let t: i32 = num(rest, "--t")?.ok_or("usage: explain needs --t")?;
    let seed = num::<u32>(rest, "--seed")?.unwrap_or(if cfg.seed == 0 { 1 } else { cfg.seed });
    let depth = num::<usize>(rest, "--depth")?.unwrap_or(3);
    if t < 1 || t as u32 > cfg.max_step {
        return Err(format!("--t must be between 1 and {}", cfg.max_step));
    }
    cfg.max_step = t as u32;
    let mut m = Model::new(&cfg, &eqs, seed);
    m.sim_name = stem_of(path);
    m.nets.sim_name = m.sim_name.clone();
    m.nets.seed = Some(seed as i64);
    m.explain = Some(crate::model::Explain { target: var.into(), t, instance: opt(rest, "--instance").map(String::from), ..Default::default() });
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.run()));
    if let Err(p) = r {
        let msg = p.downcast_ref::<String>().cloned().unwrap_or_default();
        return Err(format!("step {}: {}", m.t, msg));
    }
    match m.explain.take().and_then(|e| e.result) {
        Some(n) => Ok(json!({ "model": model, "seed": seed, "t": t, "tree": node_json(&n, depth),
            "note": "reads: the values the equation requested, in order; computed_now: its own equation ran because of this request" })),
        None => Err(format!("'{}' was not computed at step {} (it may be a parameter, written rather than computed, or the instance doesn't exist)", var, t)),
    }
}

// ------------------------------------------------------------------ new

fn ident(label: &str) -> String {
    let s: String = label.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
    format!("eq_{}", s.trim_matches('_'))
}

fn new_project(name: &str, rest: &[String]) -> Result<Value, String> {
    let from = opt(rest, "--from").ok_or("usage: new needs --from CONFIG.lsd")?;
    let cfg = load(from)?;
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err("NAME must start with a letter and use letters, digits or _".into());
    }
    let dir = PathBuf::from(opt(rest, "--dir").unwrap_or(".")).join(name.to_lowercase());
    if dir.exists() {
        return Err(format!("{} already exists", dir.display()));
    }
    std::fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
    let engine = env!("CARGO_MANIFEST_DIR");
    // a local checkout of the engine is used directly; an installed binary points the project at GitHub
    let installed = engine.contains("/.cargo/git/checkouts/") || engine.contains("/.cargo/registry/");
    let dep = if (!installed && Path::new(engine).join("Cargo.toml").exists()) || std::env::var("LSD_RS_PATH").is_ok() {
        format!("lsd = {{ path = \"{}\" }}", std::env::var("LSD_RS_PATH").unwrap_or_else(|_| engine.to_string()))
    } else {
        "lsd = { git = \"https://github.com/RJuro/lsd-rs\" }".to_string()
    };
    let crate_name = name.to_lowercase();
    std::fs::write(dir.join("Cargo.toml"), format!(
        "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{dep}\n\n[profile.release]\ndebug = 1\n")).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("src/main.rs"), format!(
        "//! {name}: an LSD model. Run `cargo run --release -- help`-style commands, e.g.\n//! `cargo run --release -- run {name} {cfgfile}`\nmod equations;\n\nfn main() {{\n    lsd::cli::main_with(|m| (m == \"{name}\").then(equations::equations), &[\"{name}\"]);\n}}\n",
        cfgfile = Path::new(from).file_name().unwrap().to_string_lossy())).map_err(|e| e.to_string())?;
    let mut all = Vec::new();
    elems(&cfg.root, &mut all);
    let mut src = format!("//! Equations of {name}: one function per variable or function in the configuration.\n//! The API mirrors LSD's macros; see https://github.com/RJuro/lsd-rs/blob/main/AGENTS.md\nuse lsd::model::{{Ctx, EqDef}};\n");
    let mut reg = Vec::new();
    for (obj, lab, kind) in all.iter().filter(|e| e.2 != Kind::Param) {
        let f = ident(lab);
        src.push_str(&format!("\n/// {lab} (object {obj}{})\nfn {f}(c: &mut Ctx) -> f64 {{\n    let _ = c;\n    panic!(\"equation for '{lab}' is not written yet\")\n}}\n",
            if *kind == Kind::Func { ", function" } else { "" }));
        reg.push(format!("        (\"{lab}\", EqDef::{}({f})),", if *kind == Kind::Func { "Function" } else { "Fn" }));
    }
    src.push_str(&format!("\npub fn equations() -> Vec<(&'static str, EqDef)> {{\n    vec![\n{}\n    ]\n}}\n", reg.join("\n")));
    std::fs::write(dir.join("src/equations.rs"), src).map_err(|e| e.to_string())?;
    let cfg_dst = dir.join(Path::new(from).file_name().unwrap());
    std::fs::copy(from, &cfg_dst).map_err(|e| e.to_string())?;
    Ok(json!({
        "project": dir.display().to_string(),
        "config": cfg_dst.display().to_string(),
        "equations_to_write": reg.len(),
        "next": [
            format!("write the equations in {}/src/equations.rs (the stubs panic until written)", dir.display()),
            format!("cargo build --release --manifest-path {}/Cargo.toml", dir.display()),
            format!("{}/target/release/{} check {} {}", dir.display(), crate_name, name, cfg_dst.display()),
        ],
    }))
}

// ------------------------------------------------------------------ compare

fn compare(model_dir: &str, rest: &[String]) -> Result<Value, String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("check.sh");
    let mut cmd = std::process::Command::new(&script);
    cmd.arg(model_dir).arg(opt(rest, "--steps").unwrap_or("200"));
    if let Some(r) = opt(rest, "--lsd-repo") {
        cmd.env("LSD_REPO", r);
    }
    if let Ok(exe) = std::env::current_exe() {
        cmd.env("RUST_BIN", exe);
    }
    let out = cmd.output().map_err(|e| format!("{}: {}", script.display(), e))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut configs = Vec::new();
    let mut detail = Vec::new();
    for line in text.lines() {
        if line.starts_with("    ") {
            detail.push(line.trim().to_string());
        } else if let Some((cfg, res)) = line.split_once(": ") {
            configs.push(json!({ "config": cfg, "identical": res.starts_with("identical") && !res.contains("CSV DIFFERS"), "result": res }));
        }
    }
    Ok(json!({
        "model_dir": model_dir,
        "all_identical": out.status.success() && !configs.is_empty(),
        "configs": configs,
        "first_difference_context": detail,
        "stderr": String::from_utf8_lossy(&out.stderr).lines().rev().take(5).collect::<Vec<_>>(),
    }))
}
