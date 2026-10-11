//! Example/Test/ReplDynMacro (fun_ReplDynMacro.cpp)
use crate::model::{Ctx, EqDef};

/// EQUATION("Fitness"): random walk of fitness, optionally scaled by the lagged share.
fn fitness(c: &mut Ctx) -> f64 {
    let v0 = c.v("DeltaFit");
    let v1 = c.vl("Fitness", 1);
    let v2 = c.rnd();
    let v5 = c.v("Drift");
    let v3 = c.v("CumulativeModel");
    if v3 == 0.0 {
        v1 + v5 + (v2 - 0.5) * 2.0 * v0 // non-cumulative model
    } else {
        let v4 = c.vl("ms", 1);
        v1 + v5 * v4 + (v2 - 0.5) * 2.0 * v0 // cumulative model
    }
}

/// EQUATION("AvFit"): average fitness weighted by Num[t-1].
fn av_fit(c: &mut Ctx) -> f64 {
    let mut v3 = 0.0;
    let mut v2 = 0.0;
    let mut cur = c.first("Species");
    while let Some(o) = cur {
        let v0 = c.vls(o, "Num", 1);
        let v1 = c.vs(o, "Fitness");
        v2 += v0 * v1;
        v3 += v0;
        cur = c.next(o);
    }
    v2 / v3
}

/// EQUATION("ms"): population share of the species.
fn ms(c: &mut Ctx) -> f64 {
    let v0 = c.v("Num");
    let v1 = c.v("TotNum");
    v0 / v1
}

/// EQUATION("Num"): replicator dynamics, Num[t-1] scaled by relative fitness, rounded.
fn num(c: &mut Ctx) -> f64 {
    let v0 = c.v("Fitness");
    let v1 = c.vl("Num", 1);
    let v2 = c.v("Alpha");
    let v3 = c.v("AvFit");
    (v1 + v1 * v2 * (v0 - v3) / v3).round()
}

/// EQUATION("TotNum"): sum of Num over species.
fn tot_num(c: &mut Ctx) -> f64 {
    let mut v0 = 0.0;
    let mut cur = c.first("Species");
    while let Some(o) = cur {
        v0 += c.vs(o, "Num");
        cur = c.next(o);
    }
    v0
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    vec![
        ("Fitness", EqDef::Fn(fitness)),
        ("AvFit", EqDef::Fn(av_fit)),
        ("ms", EqDef::Fn(ms)),
        ("Num", EqDef::Fn(num)),
        ("TotNum", EqDef::Fn(tot_num)),
    ]
}
