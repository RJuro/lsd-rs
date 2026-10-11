//! MCP server over stdio (newline-delimited JSON-RPC 2.0). Each tool is one CLI command; tools that
//! take `project` build that model project and run its binary instead of the models built in here.

use crate::cli::{dispatch, Registry};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

fn tools() -> Value {
    let s = |props: Value, req: &[&str]| json!({ "type": "object", "properties": props, "required": req });
    let project = json!({ "type": "string", "description": "Path to a model project made with lsd_new (omit for the built-in models)" });
    json!([
        { "name": "lsd_models", "description": "List the models available (built in, or in a project).",
          "inputSchema": s(json!({ "project": project }), &[]) },
        { "name": "lsd_describe", "description": "Describe an LSD configuration (.lsd): objects, instance counts, variables, parameters, lags, saved series, initial values. With `model`, also which equations are missing.",
          "inputSchema": s(json!({ "config": { "type": "string" }, "model": { "type": "string" }, "project": project }), &["config"]) },
        { "name": "lsd_check", "description": "Check a model against a configuration: every variable has an equation, and the first 3 steps run.",
          "inputSchema": s(json!({ "model": { "type": "string" }, "config": { "type": "string" }, "project": project }), &["model", "config"]) },
        { "name": "lsd_run", "description": "Run a model: one or several Monte Carlo runs (seeds seed..seed+runs-1, in parallel), optional parameter overrides. Returns a bounded summary per saved series and the CSV paths (LSD's format).",
          "inputSchema": s(json!({
              "model": { "type": "string" }, "config": { "type": "string" },
              "seed": { "type": "integer" }, "runs": { "type": "integer" }, "jobs": { "type": "integer" },
              "steps": { "type": "integer" },
              "set": { "type": "object", "additionalProperties": { "type": "number" }, "description": "Initial values to override, label -> value (all instances)" },
              "out": { "type": "string", "description": "Directory for CSV results (default ./lsd-out)" },
              "csv": { "type": "boolean", "description": "Write CSV files (default true)" },
              "project": project }), &["model", "config"]) },
        { "name": "lsd_series", "description": "Read columns of a results CSV (bounded): a name matches the column or all its instances (\"Num\" matches \"Num_1_3\").",
          "inputSchema": s(json!({ "csv": { "type": "string" }, "cols": { "type": "array", "items": { "type": "string" } },
              "from": { "type": "integer" }, "to": { "type": "integer" }, "every": { "type": "integer" }, "max_rows": { "type": "integer" } }), &["csv"]) },
        { "name": "lsd_explain", "description": "Explain one value: run to step t and return the tree of values the variable's equation read when computing it (replaces LSD's debugger).",
          "inputSchema": s(json!({ "model": { "type": "string" }, "config": { "type": "string" }, "var": { "type": "string" },
              "t": { "type": "integer" }, "instance": { "type": "string", "description": "Instance path as in CSV names, e.g. 1_3" },
              "seed": { "type": "integer" }, "depth": { "type": "integer" }, "project": project }), &["model", "config", "var", "t"]) },
        { "name": "lsd_new", "description": "Create a model project from an LSD configuration: a Rust crate with one equation stub per variable, ready to fill in and build.",
          "inputSchema": s(json!({ "name": { "type": "string" }, "from": { "type": "string" }, "dir": { "type": "string" } }), &["name", "from"]) },
        { "name": "lsd_build", "description": "Build a model project (cargo build --release); returns compiler errors if any.",
          "inputSchema": s(json!({ "project": project }), &["project"]) },
        { "name": "lsd_compare", "description": "Compare a model with the original LSD, computation by computation (needs Homebrew GCC and the LSD sources). The model folder holds LSD's C++ equation file and configurations; the Rust model must have the folder's name.",
          "inputSchema": s(json!({ "lsd_model_dir": { "type": "string" }, "steps": { "type": "integer" },
              "lsd_repo": { "type": "string" }, "project": project }), &["lsd_model_dir"]) },
    ])
}

/// Translate a tool call into CLI arguments.
fn to_args(name: &str, a: &Value) -> Result<Vec<String>, String> {
    let s = |k: &str| a.get(k).and_then(|v| v.as_str()).map(String::from);
    let req = |k: &str| s(k).ok_or_else(|| format!("missing argument '{}'", k));
    let mut v: Vec<String> = Vec::new();
    let flag = |v: &mut Vec<String>, k: &str, f: &str| {
        if let Some(x) = a.get(k).filter(|x| !x.is_null()) {
            v.push(f.into());
            v.push(x.as_str().map(String::from).unwrap_or_else(|| x.to_string()));
        }
    };
    match name {
        "lsd_models" => v.push("models".into()),
        "lsd_describe" => { v.extend(["describe".into(), req("config")?]); flag(&mut v, "model", "--model"); }
        "lsd_check" => v.extend(["check".into(), req("model")?, req("config")?]),
        "lsd_run" => {
            v.extend(["run".into(), req("model")?, req("config")?]);
            for (k, f) in [("seed", "--seed"), ("runs", "--runs"), ("jobs", "--jobs"), ("steps", "--steps"), ("out", "--out")] {
                flag(&mut v, k, f);
            }
            if let Some(m) = a.get("set").and_then(|x| x.as_object()) {
                for (k, x) in m {
                    v.extend(["--set".into(), format!("{}={}", k, x)]);
                }
            }
            if a.get("csv") == Some(&json!(false)) {
                v.push("--no-csv".into());
            }
        }
        "lsd_series" => {
            v.extend(["series".into(), req("csv")?]);
            if let Some(c) = a.get("cols").and_then(|x| x.as_array()) {
                v.extend(["--cols".into(), c.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(",")]);
            }
            for (k, f) in [("from", "--from"), ("to", "--to"), ("every", "--every"), ("max_rows", "--max-rows")] {
                flag(&mut v, k, f);
            }
        }
        "lsd_explain" => {
            v.extend(["explain".into(), req("model")?, req("config")?]);
            for (k, f) in [("var", "--var"), ("t", "--t"), ("instance", "--instance"), ("seed", "--seed"), ("depth", "--depth")] {
                flag(&mut v, k, f);
            }
        }
        "lsd_new" => { v.extend(["new".into(), req("name")?]); flag(&mut v, "from", "--from"); flag(&mut v, "dir", "--dir"); }
        "lsd_compare" => {
            v.extend(["compare".into(), req("lsd_model_dir")?]);
            flag(&mut v, "steps", "--steps");
            flag(&mut v, "lsd_repo", "--lsd-repo");
        }
        _ => return Err(format!("unknown tool '{}'", name)),
    }
    Ok(v)
}

/// Build a model project; returns the path of its binary or the compiler's errors.
fn build(project: &str) -> Result<String, String> {
    let manifest = Path::new(project).join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {}", manifest.display(), e))?;
    let name = text.lines().find_map(|l| l.trim().strip_prefix("name = \"").and_then(|r| r.strip_suffix('"')))
        .ok_or("no package name in Cargo.toml")?.to_string();
    let out = std::process::Command::new("cargo")
        .args(["build", "--release", "--quiet", "--message-format=short", "--manifest-path"]).arg(&manifest)
        .output().map_err(|e| format!("cargo: {}", e))?;
    if !out.status.success() {
        let err: Vec<String> = String::from_utf8_lossy(&out.stderr).lines().filter(|l| l.contains("error") || l.contains("-->"))
            .take(40).map(String::from).collect();
        return Err(format!("build failed:\n{}", err.join("\n")));
    }
    Ok(Path::new(project).join("target/release").join(name).display().to_string())
}

fn call(name: &str, a: &Value, get: Registry, names: &[&str]) -> Result<Value, String> {
    if name == "lsd_build" {
        let p = a.get("project").and_then(|v| v.as_str()).ok_or("missing argument 'project'")?;
        return build(p).map(|bin| json!({ "built": bin }));
    }
    let args = to_args(name, a)?;
    match a.get("project").and_then(|v| v.as_str()) {
        None => dispatch(&args, get, names),
        Some(p) => {
            let bin = build(p)?;
            let out = std::process::Command::new(&bin).args(&args).output().map_err(|e| format!("{}: {}", bin, e))?;
            let v: Value = serde_json::from_slice(&out.stdout).map_err(|_| String::from_utf8_lossy(&out.stdout).to_string())?;
            match v.get("error").and_then(|e| e.as_str()) {
                Some(e) => Err(e.to_string()),
                None => Ok(v),
            }
        }
    }
}

pub fn serve(get: Registry, names: &[&str]) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            let _ = writeln!(stdout, "{}", json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "parse error" } }));
            continue;
        };
        let Some(id) = msg.get("id").cloned() else { continue }; // a notification: no reply
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "lsd-rs", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "LSD agent-based simulation engine (Rust, bit-compatible with LSD 8.1). Start with lsd_models or lsd_describe on a .lsd configuration; run with lsd_run and read results with lsd_series; use lsd_explain to see why a value is what it is. To build a new model: lsd_new from a .lsd file, write the equations in src/equations.rs, then lsd_build and lsd_check with `project`.",
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let r = call(name, &args, get, names);
                Ok(match r {
                    Ok(v) => json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap() }], "isError": false }),
                    Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
                })
            }
            _ => Err(json!({ "code": -32601, "message": format!("method not found: {}", method) })),
        };
        let reply = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
        };
        let _ = writeln!(stdout, "{}", reply);
        let _ = stdout.flush();
    }
}
