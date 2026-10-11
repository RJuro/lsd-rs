// Bit-exact comparison of src/rng.rs against LSD's RNG logic compiled with GCC 15 (tests/rng_ref.cpp).
#[allow(dead_code)]
#[path = "../src/rng.rs"]
mod rng;

use rng::LsdRng;
use std::process::Command;

const N: usize = 200_000;

fn call(r: &mut LsdRng, op: &str, a: f64, b: f64) -> f64 {
    match op {
        "ran1" => r.ran1(),
        "uniform" => r.uniform(a, b),
        "uniform_int" => r.uniform_int(a, b),
        "rnd_int" => r.rnd_int(a as i32, b as i32) as f64,
        "norm" => r.norm(a, b),
        "lnorm" => r.lnorm(a, b),
        "gamma" => r.gamma(a, b),
        "beta" => r.beta(a, b),
        "poisson" => r.poisson(a),
        "bernoulli" => r.bernoulli(a),
        _ => panic!("unknown op {op}"),
    }
}

#[test]
fn matches_lsd_cpp_bit_for_bit() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let exe = format!("{}/rng_ref", env!("CARGO_TARGET_TMPDIR"));
    let gxx = if std::path::Path::new("/opt/homebrew/bin/g++-15").exists() { "/opt/homebrew/bin/g++-15" } else { "g++-15" };
    if Command::new(gxx).arg("--version").output().is_err() {
        eprintln!("skipped: needs GCC 15 (g++-15), whose libstdc++ is the reference for LSD's random numbers");
        return;
    }
    let st = Command::new(gxx)
        .args(["-std=c++17", "-O2", "-ffp-contract=off", "-o", &exe, &format!("{dir}/tests/rng_ref.cpp")])
        .status()
        .expect("run g++-15");
    assert!(st.success(), "compiling rng_ref.cpp failed");

    let calls_path = format!("{dir}/tests/rng_calls.txt");
    let calls: Vec<(String, f64, f64)> = std::fs::read_to_string(&calls_path).unwrap().lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut t = l.split_whitespace();
            (t.next().unwrap().to_string(), t.next().unwrap().parse().unwrap(), t.next().unwrap_or("0").parse().unwrap())
        })
        .collect();

    for seed in [1u32, 2, 12345, 0, 4294967295] {
        let out = Command::new(&exe).args([&seed.to_string(), &N.to_string(), &calls_path]).output().expect("run rng_ref");
        assert!(out.status.success(), "rng_ref failed");
        let text = String::from_utf8(out.stdout).unwrap();
        let want: Vec<f64> = text.lines().map(|l| l.parse().unwrap()).collect();
        assert_eq!(want.len(), N);
        let mut r = LsdRng::new(seed);
        for (i, w) in want.iter().enumerate() {
            let (op, a, b) = &calls[i % calls.len()];
            let got = call(&mut r, op, *a, *b);
            let same = got.to_bits() == w.to_bits() || (got.is_nan() && w.is_nan());
            assert!(same, "seed {seed}: first mismatch at draw {i}: {op}({a}, {b}) got {got:e} ({:#x}) want {w:e} ({:#x})", got.to_bits(), w.to_bits());
        }
    }
}
