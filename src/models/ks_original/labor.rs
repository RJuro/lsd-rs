//! fun_KS_labor.h: Labor market object equations.
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Effective work force (labor) size, increase workforce if full employment and flagAddWorkers is set to 1.
fn ls(c: &mut Ctx) -> f64 {
    let mut v1 = c.v("delta"); // population growth rate

    // growing workforce and demand is higher than labor supply?
    let par = c.parent();
    let cap = capsecl1(c);
    let con = consecl1(c);
    if c.vs(par, "flagAddWorkers") == 1.0 && c.vs(cap, "L1d") + c.vs(con, "L2d") > c.current() {
        v1 += 0.02; // lump grow in labor supply
    }

    c.current() * (1.0 + v1) // grow population
}

/// Centralized wage imposed to all workers. Notice 'psi1' and 'psi2' are exchanged in the original paper equation.
fn w(c: &mut Ctx) -> f64 {
    let con = consecl1(c);
    let par = c.parent();
    let v1 = c.v("psi1"); // inflation adjust. parameter
    let v2 = c.v("psi2"); // general prod. adjust. param.
    let v3 = c.v("psi3"); // unemploym. adjust. parameter
    let v5 = c.vls(con, "dCPIb", 1); // inflation variation
    let v6 = c.vls(par, "dAb", 1); // general productivity variat.
    let v7 = c.vl("dUb", 1); // unemployment variation

    let v0 = c.current() * (1.0 + v1 * v5 + v2 * v6 + v3 * v7);

    max(v0, c.v("w0min")) // ensure subsistence level
}

/// Unemployment benefit ("wage") paid by government.
fn w_u(c: &mut Ctx) -> f64 {
    let x = c.v("phi");
    x * c.v("w")
}

/// Labor aggregated demand.
fn l(c: &mut Ctx) -> f64 {
    let cap = capsecl1(c);
    let con = consecl1(c);
    let x = c.vs(cap, "L1");
    x + c.vs(con, "L2")
}

/// Total taxes paid by workers on wages.
fn tax_w(c: &mut Ctx) -> f64 {
    let par = c.parent();
    if c.vs(par, "flagTax") >= 1.0 {
        let x = c.vs(par, "tr");
        x * c.v("W")
    } else {
        0.0
    }
}

/// Unemployment rate.
fn u(c: &mut Ctx) -> f64 {
    let x = c.v("L");
    1.0 - x / c.v("Ls")
}

/// Total (nominal) wages.
fn w_total(c: &mut Ctx) -> f64 {
    let x = c.v("L");
    x * c.v("w")
}

/// Notional unemployment (bounded) rate of change. Used for wages adjustment only.
fn d_ub(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let per = c.vs(par, "mPer");
    let lim = c.vs(par, "mLim");
    let p = c.p;
    mov_avg_bound(c, p, "U", lim, per)
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("Ls", EqDef::Fn(ls)),
        ("w", EqDef::Fn(w)),
        ("wU", EqDef::Fn(w_u)),
        ("L", EqDef::Fn(l)),
        ("TaxW", EqDef::Fn(tax_w)),
        ("U", EqDef::Fn(u)),
        ("W", EqDef::Fn(w_total)),
        ("dUb", EqDef::Fn(d_ub)),
    ]);
}
