//! Example/Test/LogisticChaos (fun_lc.cpp)
use crate::model::{Ctx, EqDef};

/// EQUATION("X"): logistic map X(t) = m*X(t-1)*(1-X(t-1)); chaotic for m > 3 or so.
fn x(c: &mut Ctx) -> f64 {
    let v0 = c.vl("X", 1);
    let v1 = c.v("m");
    v1 * v0 * (1.0 - v0)
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    vec![("X", EqDef::Fn(x))]
}
