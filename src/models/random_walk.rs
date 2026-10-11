//! Example/Test/RandomWalk (fun_rwalk.cpp)
use crate::model::{Ctx, EqDef};

fn x(c: &mut Ctx) -> f64 {
    let ev = c.v("RandomEvent");
    let lag = c.vl("X", 1);
    ev + lag
}

fn random_event(c: &mut Ctx) -> f64 {
    let lo = c.v("minX");
    let hi = c.v("maxX");
    c.uniform(lo, hi)
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    vec![("X", EqDef::Fn(x)), ("RandomEvent", EqDef::Fn(random_event))]
}
