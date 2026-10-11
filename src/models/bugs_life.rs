//! Example/Test/BugsLife (fun_anitbiotics.cpp)
use crate::model::{Ctx, EqDef};

/// Time passing by.
fn age(c: &mut Ctx) -> f64 {
    let cur = c.m.var(c.var).val[0];
    cur + 1.0
}

/// Check if the bug is too old, starved, or it can mate.
fn life(c: &mut Ctx) -> f64 {
    let age = c.v("Age");
    let max_age = c.v("MaxAge");
    let res;
    if max_age <= age {
        res = 1.0;
    } else {
        let eat = c.v("Eat");
        if eat == 0.0 {
            res = 2.0;
        } else {
            res = 3.0;
            let prob = c.v("ProbMate");
            if c.rnd() < prob {
                c.write("ReadyMate", 1.0);
            } else {
                c.write("ReadyMate", 0.0);
            }
        }
    }
    res
}

/// Test if there is food (FUNCTION).
fn eat(c: &mut Ctx) -> f64 {
    let fed = c.v("Fed");
    let stock = c.vl("Stock", 1);
    let p = (stock - fed) / stock;
    if c.rnd() < p {
        c.incr("Fed", 1.0);
        1.0
    } else {
        0.0
    }
}

/// Scan all bugs and reproduce the ones ready to mate.
fn mate(c: &mut Ctx) -> f64 {
    c.v("Action");
    let mut n = 0.0;
    let mut cur = c.first("Bug");
    while let Some(o) = cur {
        let ready = c.vs(o, "ReadyMate");
        if ready > 0.0 {
            let cur2 = c.addobj_ex("Bug", o);
            c.writes(cur2, "ReadyMate", 0.0);
            let t = c.t();
            c.writels(cur2, "Age", 0.0, t);
            n += 1.0;
        }
        cur = c.next(o);
    }
    n
}

/// Main cycle on existing bugs: remove dead ones and keep statistics.
fn action(c: &mut Ctx) -> f64 {
    c.write("DeadAge", 0.0);
    c.write("DeadStarved", 0.0);
    c.write("Fed", 0.0);
    let mut alive = 0.0;
    let mut cur = c.first("Bug");
    while let Some(o) = cur {
        let next = c.next(o); // CYCLE_SAFE
        let life = c.vs(o, "Life");
        if life == 2.0 || life == 1.0 {
            c.delete(o);
            if life == 1.0 {
                c.incr("DeadAge", 1.0);
            }
            if life == 2.0 {
                c.incr("DeadStarved", 1.0);
            }
        } else {
            alive += 1.0;
        }
        cur = next;
    }
    alive
}

/// Compute the amount of food.
fn stock(c: &mut Ctx) -> f64 {
    c.v("Action");
    let fed = c.v("Fed");
    let incr = c.v("IncrStock");
    let new_stock = c.v("NewStock");
    let cur = c.m.var(c.var).val[0];
    let left = cur - fed;
    left * incr + (1.0 - incr) * new_stock
}

/// Survivors from the previous generation plus new offspring.
fn num_bugs(c: &mut Ctx) -> f64 {
    let a = c.v("Action");
    let m = c.v("Mate");
    a + m
}

/// Probability of mating by age (FUNCTION).
fn prob_mate(c: &mut Ctx) -> f64 {
    let bug = c.c.unwrap();
    let age = c.vs(bug, "Age");
    let step = c.v("StepAge");
    if age > 0.0 {
        1.0 - step * (age - 1.0)
    } else {
        0.0
    }
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    vec![
        ("Age", EqDef::Fn(age)),
        ("Life", EqDef::Fn(life)),
        ("Eat", EqDef::Function(eat)),
        ("Mate", EqDef::Fn(mate)),
        ("Action", EqDef::Fn(action)),
        ("Stock", EqDef::Fn(stock)),
        ("NumBugs", EqDef::Fn(num_bugs)),
        ("ProbMate", EqDef::Function(prob_mate)),
    ]
}
