//! fun_KS_financial.h: Financial market object equations.
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Total bank losses from bad debt. This variable must be explicitly recalculated after entry/exit.
fn bad_deb(c: &mut Ctx) -> f64 {
    let x = c.v("BadDeb1");
    x + c.v("BadDeb2")
}

/// Bank bad debt (defaults) from capital-good sector. Just reset once per period, updated in 'entry1exit'.
fn bad_deb1(_c: &mut Ctx) -> f64 {
    0.0
}

/// Bank bad debt (defaults) from consumption-good sector. Just reset once per period, updated in 'entry2exit'.
fn bad_deb2(_c: &mut Ctx) -> f64 {
    0.0
}

/// Bank deposits. Net deposits from exiting and entering firms in period not considered.
fn depo(c: &mut Ctx) -> f64 {
    let par = c.parent();
    c.vs(par, "Sav"); // ensure savings are calculated
    c.v("Loans"); // ensure all transactions done
    let cap = capsecl1(c);
    let con = consecl1(c);
    let x = c.vs(par, "SavAcc") + c.vs(cap, "NW1");
    x + c.vs(con, "NW2")
}

/// Government deposits.
fn depo_g(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let cur = c.current();
    let x = cur - c.vs(par, "Def") - c.vls(par, "Deb", 1);
    max(x, 0.0)
}

/// Bank loans (non-defaulted). Net loans to exiting and entering firms in period not considered.
fn loans(c: &mut Ctx) -> f64 {
    let cap = capsecl1(c);
    let con = consecl1(c);
    c.vs(cap, "Tax1"); // ensure transactions are done
    c.vs(con, "Tax2");
    let x = c.vs(cap, "Deb1");
    x + c.vs(con, "Deb2")
}

/// Bank profits (losses).
fn pi_b(c: &mut Ctx) -> f64 {
    let x = c.v("iB") - c.v("iDb");
    x - c.vl("BadDeb", 1)
}

/// Bank interest income from loans.
fn i_b(c: &mut Ctx) -> f64 {
    let cap = capsecl1(c);
    let con = consecl1(c);
    let r = c.v("r");
    let s = c.sumls(cap, "_Deb1", 1) + c.sumls(con, "_Deb2", 1);
    r * s
}

/// Bank interest payments from deposits.
fn i_db(c: &mut Ctx) -> f64 {
    let cap = capsecl1(c);
    let con = consecl1(c);
    let par = c.parent();
    let r = c.v("r");
    let s = c.vls(par, "SavAcc", 1) + c.sumls(cap, "_NW1", 1) + c.sumls(con, "_NW2", 1);
    r * s
}

pub fn register(r: &mut Reg) {
    // 'DepoG' also has EQUATION_DUMMY( "DepoG", "Deb" ) in the C++; the first definition (the real equation) wins
    r.extend([
        ("BadDeb", EqDef::Fn(bad_deb)),
        ("BadDeb1", EqDef::Fn(bad_deb1)),
        ("BadDeb2", EqDef::Fn(bad_deb2)),
        ("Depo", EqDef::Fn(depo)),
        ("DepoG", EqDef::Fn(depo_g)),
        ("Loans", EqDef::Fn(loans)),
        ("PiB", EqDef::Fn(pi_b)),
        ("iB", EqDef::Fn(i_b)),
        ("iDb", EqDef::Fn(i_db)),
    ]);
}
