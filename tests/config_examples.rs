// Builds config.rs independently of lib.rs; parses every example .lsd file.
#[path = "../src/config.rs"]
#[allow(dead_code)]
mod config;

use config::*;
use std::path::{Path, PathBuf};


fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "lsd") {
            out.push(p);
        }
    }
}

fn check(o: &ObjSpec, parent_total: usize) {
    assert_eq!(o.counts.len(), parent_total, "{}: one count per parent instance", o.label);
    let total = o.total();
    for e in &o.elems {
        assert_eq!(e.values.len(), total, "{}.{}", o.label, e.label);
        for v in &e.values {
            match e.kind {
                Kind::Param => assert_eq!(v.len(), 1),
                _ => assert_eq!(v.len(), e.num_lag),
            }
        }
    }
    for s in &o.sons {
        check(s, total);
    }
}

fn run(dir: &str) {
    let mut files = vec![];
    collect(Path::new(dir), &mut files);
    files.sort();
    assert!(!files.is_empty(), "no .lsd files under {dir}");
    let mut failed = vec![];
    for f in &files {
        match load(f) {
            Ok(c) => {
                assert_eq!(c.root.counts, vec![1]);
                check(&c.root, 1);
            }
            Err(e) => failed.push(e),
        }
    }
    println!("{dir}: {} parsed, {} failed", files.len() - failed.len(), failed.len());
    assert!(failed.is_empty(), "failed:\n{}", failed.join("\n"));
}

/// Every configuration shipped with LSD (needs LSD's sources: LSD_REPO, as for check.sh; skipped otherwise).
#[test]
fn config_parses_all_examples() {
    let repo = std::env::var("LSD_REPO").unwrap_or_else(|_| "target/lsd-src".into());
    let dir = format!("{repo}/Example");
    if !Path::new(&dir).is_dir() {
        eprintln!("skipped: no LSD sources at {dir} (set LSD_REPO, or run ./check.sh once to fetch them)");
        return;
    }
    run(&dir);
}

/// The test models written for this engine.
#[test]
fn config_parses_test_models() {
    run("tests/lsd-models");
}
