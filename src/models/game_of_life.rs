//! Example/Test/GameOfLife (fun_life.cpp)
use crate::model::{Ctx, EqDef};

/// Choose (FUNCTION): the cell `c` picks an option with probability proportional to its neighbours' past choices.
fn choose(c: &mut Ctx) -> f64 {
    let cell = c.c.expect("Choose needs a caller");
    let minprob = c.v("minprob");
    let mut cur = c.first("Option");
    while let Some(o) = cur {
        c.writes(o, "num", minprob);
        cur = c.next(o);
    }

    let unit = c.v("UnitProb");
    let prev = c.vls(cell, "State", 1);
    let o = c.search_cnd("Id", prev).unwrap();
    c.incrs(o, "num", unit);

    let right = c.v_cheat("right", cell);
    let o = c.search_cnd("Id", right).unwrap();
    c.incrs(o, "num", unit);

    let left = c.v_cheat("left", cell);
    let o = c.search_cnd("Id", left).unwrap();
    c.incrs(o, "num", unit);

    let up = c.v_cheat("up", cell);
    let o = c.search_cnd("Id", up).unwrap();
    c.incrs(o, "num", unit);

    let down = c.v_cheat("down", cell);
    let o = c.search_cnd("Id", down).unwrap();
    c.incrs(o, "num", unit);

    let elas = c.v("Elas");
    let mut cur = c.first("Option");
    while let Some(o) = cur {
        let num = c.vs(o, "num");
        c.writes(o, "prob", num.powf(elas));
        cur = c.next(o);
    }

    let chosen = c.rnddraw("Option", "prob").unwrap();
    let id = c.vs(chosen, "Id");
    if id != prev {
        c.incrs(chosen, "glob", 1.0);
        let o = c.search_cnd("Id", prev).unwrap();
        c.incrs(o, "glob", -1.0);
    }
    id
}

/// State: the option chosen by Choose (the lattice display update is a no-op in the headless build).
fn state(c: &mut Ctx) -> f64 {
    let v0 = c.v("Choose");
    let v3 = c.vl("State", 1);
    if v0 != v3 {
        c.v("Line");
        c.v("Col");
    }
    v0
}

/// down (FUNCTION): previous state of the cell one line below (wrapping).
fn down(c: &mut Ctx) -> f64 {
    let cell = c.c.expect("down needs a caller");
    let col = c.vs(cell, "Col");
    let line = c.vs(cell, "Line");
    let nline = c.v("NLine");
    let row = if line == nline { c.search_cnd("Line", 1.0) } else { c.search_cnd("Line", line + 1.0) }.unwrap();
    let cell1 = c.search_cnds(row, "Col", col).unwrap();
    c.vls(cell1, "State", 1)
}

/// up (FUNCTION): previous state of the cell one line above (wrapping).
fn up(c: &mut Ctx) -> f64 {
    let cell = c.c.expect("up needs a caller");
    let col = c.vs(cell, "Col");
    let line = c.vs(cell, "Line");
    let nline = c.v("NLine");
    let row = if line == 1.0 { c.search_cnd("Line", nline) } else { c.search_cnd("Line", line - 1.0) }.unwrap();
    let cell1 = c.search_cnds(row, "Col", col).unwrap();
    c.vls(cell1, "State", 1)
}

/// left (FUNCTION): previous state of the cell one column to the left (wrapping).
fn left(c: &mut Ctx) -> f64 {
    let cell = c.c.expect("left needs a caller");
    let col = c.vs(cell, "Col");
    let _line = c.vs(cell, "Line");
    let ncol = c.v("NCol");
    let row = c.up(cell);
    let target = if col == 1.0 { ncol } else { col - 1.0 };
    let cell1 = c.search_cnds(row, "Col", target).unwrap();
    c.vls(cell1, "State", 1)
}

/// right (FUNCTION): previous state of the cell one column to the right (wrapping).
fn right(c: &mut Ctx) -> f64 {
    let cell = c.c.expect("right needs a caller");
    let col = c.vs(cell, "Col");
    let _line = c.vs(cell, "Line");
    let ncol = c.v("NCol");
    let row = c.up(cell);
    let target = if col == ncol { 1.0 } else { col + 1.0 };
    let cell1 = c.search_cnds(row, "Col", target).unwrap();
    c.vls(cell1, "State", 1)
}

/// Init: builds the NLine x NCol grid of cells with random states, then turns itself into a parameter.
fn init(c: &mut Ctx) -> f64 {
    let ncol = c.v("NCol");
    let nline = c.v("NLine");
    let nstate = c.v("NState");

    let v3 = c.uniform_int(0.0, nstate - 1.0);
    let cell = c.search("Cell").unwrap();
    c.writels(cell, "State", v3, 1.0);
    let o = c.search_cnd("Id", v3).unwrap();
    c.incrs(o, "glob", 1.0);

    let row = c.search("Row").unwrap();
    let mut j = 1;
    while j < ncol as i32 {
        let cell2 = c.addobjs_ex(row, "Cell", cell);
        c.writes(cell2, "Col", (j + 1) as f64);
        let s = c.uniform_int(0.0, nstate - 1.0);
        c.writels(cell2, "State", s, 1.0);
        let o = c.search_cnd("Id", s).unwrap();
        c.incrs(o, "glob", 1.0);
        j += 1;
    }

    let row = c.search("Row").unwrap();
    let mut i = 1;
    while i < nline as i32 {
        let row1 = c.addobj_ex("Row", row);
        c.writes(row1, "Line", (i + 1) as f64);
        let mut cur = c.firsts(row1, "Cell");
        while let Some(cell2) = cur {
            let s = c.uniform_int(0.0, nstate - 1.0);
            c.writels(cell2, "State", s, 1.0);
            let o = c.search_cnd("Id", s).unwrap();
            c.incrs(o, "glob", 1.0);
            cur = c.next(cell2);
        }
        i += 1;
    }

    // init_lattice(...) is display-only; PARAMETER: var->param = 1
    c.parameter();
    1.0
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    vec![
        ("Choose", EqDef::Function(choose)),
        ("State", EqDef::Fn(state)),
        ("down", EqDef::Function(down)),
        ("up", EqDef::Function(up)),
        ("left", EqDef::Function(left)),
        ("right", EqDef::Function(right)),
        ("Init", EqDef::Fn(init)),
    ]
}
