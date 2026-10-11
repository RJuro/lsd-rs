//! fun_KS_stats.h: statistics-only equations (not required for the model to run).
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

// ---- country-level stats (object Mac)

/// Total credit demand.
fn cd(c: &mut Ctx) -> f64 {
    let s = secstal2(c);
    let x = c.vs(s, "CD1");
    x + c.vs(s, "CD2")
}

/// Total credit demand constraint.
fn cdc(c: &mut Ctx) -> f64 {
    let s = secstal2(c);
    let x = c.vs(s, "CD1c");
    x + c.vs(s, "CD2c")
}

/// Total credit supplied.
fn cs(c: &mut Ctx) -> f64 {
    let s = secstal2(c);
    let x = c.vs(s, "CS1");
    x + c.vs(s, "CS2")
}

/// Government deficit on GDP ratio.
fn def_gdp(c: &mut Ctx) -> f64 {
    let gp = c.grandparent();
    let x = c.vs(gp, "Def");
    x / c.vs(gp, "GDPnom")
}

/// Gross domestic income (nominal terms).
fn gdi(c: &mut Ctx) -> f64 {
    let lab = labsupl2(c);
    let cap = capsecl2(c);
    let con = consecl2(c);
    let fin = finsecl2(c);
    let gp = c.grandparent();
    let mut x = c.vs(lab, "W");
    x += c.vs(cap, "Pi1");
    x += c.vs(con, "Pi2");
    x += c.vs(fin, "PiB");
    x += c.vs(gp, "Div");
    x += c.vs(gp, "G");
    x -= c.vs(gp, "Tax");
    let p = c.vs(cap, "PPI") * c.vs(con, "SI");
    x + p / c.vs(con, "m2")
}

/// Overall labor productivity growth rate.
fn d_a(c: &mut Ctx) -> f64 {
    let gp = c.grandparent();
    let v1 = c.vls(gp, "A", 1);
    if v1 > 0.0 { c.vs(gp, "A") / v1 - 1.0 } else { 0.0 }
}

// ---- financial sector stats

/// Accumulated losses from bad debt in financial sector.
fn bad_deb_acc(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    c.current() + c.vs(fin, "BadDeb")
}

/// Firms financial fragility defined as the ratio between accumulated bad debt (loans in default) and total bank
/// assets (stock of loans).
fn bda(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    // losses with bad debt, discarding negative losses (proceedings)
    let v1 = max(c.vls(fin, "BadDeb", 1), 0.0);
    // stock of non-defaulted loans
    let v2 = c.vls(fin, "Loans", 1);

    if (v1 + v2) > 0.0 { v1 / (v1 + v2) } else { 0.0 }
}

// ---- capital-good sector stats

/// Average labor productivity of machines supplied by capital-good sector.
fn atau_avg(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.aves(cap, "_Atau")
}

/// Average labor productivity of machines produced by capital-good sector.
fn btau_avg(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.aves(cap, "_Btau")
}

/// Total credit demand of firms in capital-good sector.
fn cd1(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.sums(cap, "_CD1")
}

/// Total credit demand constraint of firms in capital-good sector.
fn cd1c(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.sums(cap, "_CD1c")
}

/// Total credit supplied to firms in capital-good sector.
fn cs1(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.sums(cap, "_CS1")
}

/// Total maximum prudential credit supplied to firms in capital-good sector.
fn deb1max(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.sums(cap, "_Deb1max")
}

/// Number of historical clients of capital-good firms.
fn hc_avg(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.aves(cap, "_HC")
}

/// Normalized Herfindahl-Hirschman index for capital-good sector.
fn hh1(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    let i = c.counts(cap, "Firm1") as i32;
    if i > 1 {
        let id = i as f64;
        let x = (c.whtaves(cap, "_f1", "_f1") - 1.0 / id) / (1.0 - 1.0 / id);
        max(0.0, x)
    } else {
        1.0
    }
}

/// Hymer-Pashigian index for capital-good sector.
fn hp1(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    let mut v0 = 0.0; // index accumulator
    let mut cur = c.firsts(cap, "Firm1");
    while let Some(o) = cur {
        let a = c.vls(o, "_f1", 1) - c.vs(o, "_f1");
        v0 += a.abs(); // sum share changes
        cur = c.next(o);
    }
    v0
}

/// Number of new clients of capital-good firms.
fn nc_avg(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.aves(cap, "_NC")
}

/// R&D expenditure of capital-good sector.
fn rd(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.sums(cap, "_RD")
}

/// Average age of firms in capital-good sector.
fn age1avg(c: &mut Ctx) -> f64 {
    let cap = capsecl2(c);
    c.t() - c.aves(cap, "_t1ent")
}

// ---- consumer-good sector stats

/// Standard deviation of machine-level log labor productivity of firms in consumption-good sector.
fn a2sd(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 3];
    let con = consecl2(c);

    v[1] = c.vs(con, "A2"); // average productivity
    if v[1] <= 0.0 {
        return 0.0; // probably no production yet
    }

    v[1] = (v[1] + 1.0).ln(); // average log productivity

    let mut i = 0; // valid cases count
    v[0] = 0.0; // square difference accumulator
    let mut cur = c.firsts(con, "Firm2");
    while let Some(o) = cur {
        v[2] = c.vs(o, "_A2");
        if v[2].is_finite() && v[2] > 0.0 {
            let d = (v[2] + 1.0).ln() - v[1];
            v[0] += d * d; // pow( .., 2 )
            i += 1;
        }
        cur = c.next(o);
    }

    if i > 0 { (v[0] / i as f64).sqrt() } else { 0.0 }
}

/// Total credit demand of firms in consumption-good sector.
fn cd2(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.sums(con, "_CD2")
}

/// Total credit demand constraint of firms in consumption-good sector.
fn cd2c(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.sums(con, "_CD2c")
}

/// Total credit supplied to firms in consumption-good sector.
fn cs2(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.sums(con, "_CS2")
}

/// Total maximum prudential credit supplied to firms in consumption-good sector.
fn deb2max(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.sums(con, "_Deb2max")
}

/// Total desired expansion investment in consumption-good sector.
fn eid(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.sums(con, "_EId")
}

/// Normalized Herfindahl-Hirschman index for consumption-good sector.
fn hh2(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    let i = c.counts(con, "Firm2") as i32;
    if i > 1 {
        let id = i as f64;
        let x = (c.whtaves(con, "_f2", "_f2") - 1.0 / id) / (1.0 - 1.0 / id);
        max(0.0, x)
    } else {
        1.0
    }
}

/// Hymer-Pashigian index for consumption-good sector.
fn hp2(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    let mut v0 = 0.0; // index accumulator
    let mut cur = c.firsts(con, "Firm2");
    while let Some(o) = cur {
        let a = c.vls(o, "_f2", 1) - c.vs(o, "_f2");
        v0 += a.abs(); // sum share changes
        cur = c.next(o);
    }
    v0
}

/// Number of workers of largest firm in consumption-good sector.
fn l2larg(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.maxs(con, "_L2")
}

/// Machine (planned) scrapping rate in consumption-good sector.
fn rs2(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    let v1 = c.vls(con, "K", 1);
    if c.t() > 1.0 && v1 > 0.0 {
        let x = c.sums(con, "_RS2");
        x / (v1 / c.vs(con, "m2"))
    } else {
        0.0
    }
}

/// Total desired substitution investment in consumption-good sector.
fn sid(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.sums(con, "_SId")
}

/// Average age of firms in consumption-good sector.
fn age2avg(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.t() - c.aves(con, "_t2ent")
}

/// Change in total inventories (real terms).
fn dn(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    let x = c.vs(con, "N");
    x - c.vls(con, "N", 1)
}

/// Weighted average mark-up of consumption-good sector.
fn mu2avg(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.whtaves(con, "_mu2", "_f2")
}

/// Average number of machine brochures available to firms in consumer-good sector.
fn n_broch_avg(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    let mut v0 = 0.0;
    let mut cur = c.firsts(con, "Firm2");
    while let Some(o) = cur {
        v0 += c.counts(o, "Broch");
        cur = c.next(o);
    }
    v0 / c.vs(con, "F2")
}

// ---- labor stats

/// Effective vacancy rate (unfilled positions over total labor supply).
fn vac(c: &mut Ctx) -> f64 {
    if c.t() > 1.0 {
        let cap = capsecl2(c);
        let con = consecl2(c);
        let lab = labsupl2(c);
        let x = (c.vs(cap, "JO1") + c.vs(con, "JO2")) / c.vs(lab, "Ls");
        min(x, 1.0)
    } else {
        0.0
    }
}

/// Nominal average wage growth rate.
fn dw(c: &mut Ctx) -> f64 {
    let lab = labsupl2(c);
    let x = c.vs(lab, "w").ln();
    x - c.vls(lab, "w", 1).ln()
}

/// Centralized real wage imposed to all workers.
fn w_real(c: &mut Ctx) -> f64 {
    let lab = labsupl2(c);
    let con = consecl2(c);
    let x = c.vs(lab, "w");
    x / c.vs(con, "CPI")
}

// ---- agent-level stats

/// Machine-level effective productivity of firm in consumption-good sector.
fn a2e(c: &mut Ctx) -> f64 {
    let i = c.v("_L2") as i32;
    if i > 0 { c.v("_Q2e") / i as f64 } else { c.current() }
}

/// Number of machines to scrap of firm in consumption-good sector.
fn rs2_firm(c: &mut Ctx) -> f64 {
    let mut v0 = 0.0;
    let mut cur = c.first("Vint");
    while let Some(o) = cur {
        v0 += abs(c.vs(o, "__RSvint"));
        cur = c.next(o);
    }
    v0
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("CD", EqDef::Fn(cd)),
        ("CDc", EqDef::Fn(cdc)),
        ("CS", EqDef::Fn(cs)),
        ("DefGDP", EqDef::Fn(def_gdp)),
        ("GDI", EqDef::Fn(gdi)),
        ("dA", EqDef::Fn(d_a)),
        ("BadDebAcc", EqDef::Fn(bad_deb_acc)),
        ("Bda", EqDef::Fn(bda)),
        ("AtauAvg", EqDef::Fn(atau_avg)),
        ("BtauAvg", EqDef::Fn(btau_avg)),
        ("CD1", EqDef::Fn(cd1)),
        ("CD1c", EqDef::Fn(cd1c)),
        ("CS1", EqDef::Fn(cs1)),
        ("Deb1max", EqDef::Fn(deb1max)),
        ("HCavg", EqDef::Fn(hc_avg)),
        ("HH1", EqDef::Fn(hh1)),
        ("HP1", EqDef::Fn(hp1)),
        ("NCavg", EqDef::Fn(nc_avg)),
        ("RD", EqDef::Fn(rd)),
        ("age1avg", EqDef::Fn(age1avg)),
        ("A2sd", EqDef::Fn(a2sd)),
        ("CD2", EqDef::Fn(cd2)),
        ("CD2c", EqDef::Fn(cd2c)),
        ("CS2", EqDef::Fn(cs2)),
        ("Deb2max", EqDef::Fn(deb2max)),
        ("EId", EqDef::Fn(eid)),
        ("HH2", EqDef::Fn(hh2)),
        ("HP2", EqDef::Fn(hp2)),
        ("L2larg", EqDef::Fn(l2larg)),
        ("RS2", EqDef::Fn(rs2)),
        ("SId", EqDef::Fn(sid)),
        ("age2avg", EqDef::Fn(age2avg)),
        ("dN", EqDef::Fn(dn)),
        ("mu2avg", EqDef::Fn(mu2avg)),
        ("nBrochAvg", EqDef::Fn(n_broch_avg)),
        ("V", EqDef::Fn(vac)),
        ("dw", EqDef::Fn(dw)),
        ("wReal", EqDef::Fn(w_real)),
        ("_A2e", EqDef::Fn(a2e)),
        ("_RS2", EqDef::Fn(rs2_firm)),
        // EQUATION_DUMMY( "exit1fail", "" ) and ( "exit2fail", "" )
        ("exit1fail", EqDef::Fn(super::dummy)),
        ("exit2fail", EqDef::Fn(super::dummy)),
    ]);
}
