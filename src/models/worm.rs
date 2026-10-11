//! Example/Test/Worm (fun_Worm.cpp)
use crate::model::{Ctx, EqDef};

/// EQUATION("init"): lattice setup (graphics only, a no-op without a display), then turn
/// this variable into a parameter (PARAMETER macro: var->param = 1).
fn init(c: &mut Ctx) -> f64 {
    let _v0 = c.v("Nrow");
    let _v1 = c.v("Ncol");
    // init_lattice(...) draws only; no effect on values
    let mut cur = c.first("Segment");
    while let Some(o) = cur {
        let _v2 = c.vls(o, "row", 1);
        let _v3 = c.vls(o, "col", 1);
        // update_lattice(...) draws only
        cur = c.next(o);
    }
    c.parameter();
    1.0
}

/// EQUATION("Wait"): busy-wait loop in C++ (no effect on values); returns its lag.
fn wait(c: &mut Ctx) -> f64 {
    c.vl("Wait", 1)
}

/// EQUATION("Move"): redraw the head and erase the tail on the lattice (graphics only).
fn move_(c: &mut Ctx) -> f64 {
    let v4 = c.v("IdSegment");
    if v4 == 1.0 {
        let _v0 = c.v("row");
        let _v1 = c.v("col");
    } else {
        let v5 = c.v("NSegment");
        if v5 == v4 {
            let _v2 = c.vl("row", 1);
            let _v3 = c.vl("col", 1);
        }
    }
    1.0
}

/// EQUATION("Direction"): head turns randomly (-1, 0, 1); other segments follow the previous one.
fn direction(c: &mut Ctx) -> f64 {
    let v0 = c.v("IdSegment");
    let mut v3;
    if v0 == 1.0 {
        let v1 = c.uniform_int(-1.0, 1.0);
        let v2 = c.vl("Direction", 1);
        v3 = v2 + v1;
        if v3 < 0.0 {
            v3 += 7.0;
        } else if v3 > 7.0 {
            v3 -= 7.0;
        }
    } else {
        let w = c.parent();
        let cur = c.search_cnds(w, "IdSegment", v0 - 1.0).expect("SEARCH_CNDS found nothing");
        v3 = c.vls(cur, "Direction", 1);
    }
    v3
}

/// EQUATION("row"): row position, wrapping around at Nrow.
fn row(c: &mut Ctx) -> f64 {
    let v0 = c.v("Direction");
    let v1 = c.vl("row", 1);
    let mut v2 = if v0 == 0.0 || v0 == 1.0 || v0 == 7.0 {
        v1 + 1.0
    } else if v0 == 3.0 || v0 == 4.0 || v0 == 5.0 {
        v1 - 1.0
    } else {
        v1
    };
    let v3 = c.v("Nrow");
    if v2 > v3 {
        v2 = 1.0;
    } else if v2 < 1.0 {
        v2 = v3;
    }
    v2
}

/// EQUATION("col"): column position, wrapping around at Ncol.
fn col(c: &mut Ctx) -> f64 {
    let v0 = c.v("Direction");
    let v1 = c.vl("col", 1);
    let mut v2 = if v0 == 1.0 || v0 == 2.0 || v0 == 3.0 {
        v1 + 1.0
    } else if v0 == 5.0 || v0 == 6.0 || v0 == 7.0 {
        v1 - 1.0
    } else {
        v1
    };
    let v3 = c.v("Ncol");
    if v2 > v3 {
        v2 = 1.0;
    } else if v2 < 1.0 {
        v2 = v3;
    }
    v2
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    vec![
        ("init", EqDef::Fn(init)),
        ("Wait", EqDef::Fn(wait)),
        ("Move", EqDef::Fn(move_)),
        ("Direction", EqDef::Fn(direction)),
        ("row", EqDef::Fn(row)),
        ("col", EqDef::Fn(col)),
    ]
}
