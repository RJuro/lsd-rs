//! fun_KS_vintage.h: Vint object equations.
use crate::model::{Ctx, EqDef};

use super::class::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Labor required for desired utilization of vintage.
fn ld_vint(c: &mut Ctx) -> f64 {
    let x = c.v("__toUseVint");
    let gp = c.grandparent();
    let m2 = c.vs(gp, "m2");
    x * m2 / c.v("__Avint")
}

/// Number of machines to scrap in vintage of firm in consumption-good sector.
/// Positive values indicate non-economical machines but still in technical life.
/// Negative values represent machines out of technical life to be scrapped ASAP.
fn rs_vint(c: &mut Ctx) -> f64 {
    let gp = c.grandparent();
    let t = c.t();

    if c.v("__tVint") < t - c.vs(gp, "eta") {
        // out of technical life?
        return -c.v("__nVint"); // scrap if not in use
    }

    let par = c.parent();
    c.vs(par, "_supplier"); // ensure supplier is selected
    let cur = c.up(c.shooks(c.hooks(par, SUPPL).expect("brochure")).expect("client entry")); // pointer to supplier
    let lab = labsupl3(c);
    let v1 = c.vs(lab, "w"); // firm wage

    // unit cost advantage of new machines
    let v2 = v1 / c.v("__Avint") - v1 / c.vs(cur, "_Atau");

    // if new machine cost is not better in absolute terms or
    // payback period of replacing current vintage is over b
    if v2 <= 0.0 || c.vs(cur, "_p1") / c.vs(gp, "m2") / v2 > c.vs(gp, "b") {
        return 0.0; // nothing to scrap
    }

    c.v("__nVint") // scrap if can be replaced
}

/// Vintage production with available workers. It is capped by the machines physical capacity.
fn q_vint(c: &mut Ctx) -> f64 {
    let par = c.parent();
    c.vs(par, "_alloc2"); // ensure allocation is done
    let x = c.v("__Avint");
    x * c.v("__Lvint")
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("__LdVint", EqDef::Fn(ld_vint)),
        ("__RSvint", EqDef::Fn(rs_vint)),
        ("__Qvint", EqDef::Fn(q_vint)),
        ("__Lvint", EqDef::Fn(super::dummy)),
    ]);
}
