//! fun_KS_country.h: Country object equations and the country-level initialization.
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Nominal (monetary terms) desired aggregated consumption.
fn cd(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 3];

    // workers' net income after taxes
    // wages and unemployment bonuses this period and past period dividends
    let lab = labsupl0(c);
    v[0] = c.vs(lab, "W") + c.v("G") - c.vs(lab, "TaxW") + c.vl("Div", 1) - c.v("TaxDiv");

    // handle accumulated forced savings from the past
    match c.v("flagCons") as i32 {
        0 => {} // ignore unfilled past demand
        1 => {
            // spend all savings, deducted from entry/exit public cost/credit
            v[0] += c.v("SavAcc");
            c.write("SavAcc", 0.0);
        }
        _ => {
            // slow spend of unfulfilled past consumption is necessary to avoid
            // economy overheating, so recover up to a limit of current consumption
            v[1] = c.v("SavAcc"); // accumulated forced savings
            v[2] = v[0] * c.v("Crec"); // max recover limit

            if v[1] <= v[2] {
                // fit in limit?
                v[0] += v[1]; // yes: use all savings
                c.write("SavAcc", 0.0);
            } else {
                v[0] += v[2]; // no: spend the limit
                c.incr("SavAcc", -v[2]); // update forced savings
            }
        }
    }

    v[0]
}

/// Government expenditure (exogenous demand).
fn g(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 4];
    let lab = labsupl0(c);
    let fin = finsecl0(c);

    let i = c.v("flagGovExp") as i32; // type of govt. exped.
    v[2] = c.vs(lab, "Ls") - c.vs(lab, "L"); // unemployed workers
    v[3] = c.vls(fin, "DepoG", 1); // accumulated surplus

    if i < 2 {
        // work-or-die + min
        v[0] = v[2] * c.vs(lab, "w0min"); // minimum income
    } else {
        v[0] = v[2] * c.vs(lab, "wU"); // pay unemployment benefit
    }

    if i == 1 {
        v[0] += (1.0 + c.v("gG")) * c.current();
    }

    if v[3] > 0.0 && i == 3 {
        // spend accumulated surplus
        v[0] += min(v[3], max(0.0, -c.vl("Def", 1))); // limit to cur. deficit
    }

    v[0]
}

/// Accumulated nominal consumption from previous periods (forced savings). Also updated in 'Cd', 'Sav'.
fn sav_acc(c: &mut Ctx) -> f64 {
    // apply interest to the savings balance
    let fin = finsecl0(c);
    let mut v0 = c.current() * (1.0 + c.vs(fin, "r")); // update savings

    // add entry (equity) / exit (net cash) cost/credit incurred by households
    v0 += -c.vl("cEntry", 1) + c.vl("cExit", 1);

    round_tol(v0, 0.0, 0.001) // avoid rounding errors on zero
}

/// Overall labor productivity.
fn a(c: &mut Ctx) -> f64 {
    let x = c.v("GDPreal");
    let lab = labsupl0(c);
    x / c.vs(lab, "L")
}

/// Nominal (monetary terms) aggregated consumption.
fn cc(c: &mut Ctx) -> f64 {
    let con = consecl0(c);
    c.vs(con, "S2")
}

/// Real (in initial prices terms) aggregated consumption.
fn creal(c: &mut Ctx) -> f64 {
    let con = consecl0(c);
    let q = c.vs(con, "Q2e");
    q * c.vs(con, "pC0")
}

/// Accumulated government debt.
fn deb(c: &mut Ctx) -> f64 {
    let fin = finsecl0(c);
    let cur = c.current();
    // max( CURRENT + V( "Def" ) - VLS( .. ), 0 ): the argument expression is evaluated left to right
    let x = cur + c.v("Def") - c.vls(fin, "DepoG", 1);
    max(x, 0.0)
}

/// Government debt on GDP ratio.
fn deb_gdp(c: &mut Ctx) -> f64 {
    let x = c.v("Deb");
    x / c.v("GDPnom")
}

/// Government total deficit (negative if surplus).
fn def(c: &mut Ctx) -> f64 {
    let x = c.v("DefP");
    let fin = finsecl0(c);
    x - c.vs(fin, "PiB")
}

/// Government primary deficit (negative if surplus).
fn def_p(c: &mut Ctx) -> f64 {
    let x = c.v("G");
    x - c.v("Tax")
}

/// Government primary deficit on GDP ratio.
fn def_p_gdp(c: &mut Ctx) -> f64 {
    let x = c.v("DefP");
    x / c.v("GDPnom")
}

/// Total dividends paid by firms and banks.
fn div(c: &mut Ctx) -> f64 {
    let cap = capsecl0(c);
    let con = consecl0(c);
    let x = c.vs(cap, "Div1");
    x + c.vs(con, "Div2")
}

/// Total firm equity hold by workers/households.
fn eq(c: &mut Ctx) -> f64 {
    let cap = capsecl0(c);
    let con = consecl0(c);
    let x = c.vs(cap, "Eq1");
    x + c.vs(con, "Eq2")
}

/// Real (in initial prices terms) gross domestic product.
fn gdp_real(c: &mut Ctx) -> f64 {
    let con = consecl0(c);
    let x = c.vs(con, "Ireal") + c.v("Creal");
    max(x, 1.0)
}

/// Gross domestic product (nominal/currency terms).
fn gdp_nom(c: &mut Ctx) -> f64 {
    let con = consecl0(c);
    let x = c.v("C") + c.vs(con, "Inom") + c.vs(con, "dNnom");
    max(x, 1.0)
}

/// Residual nominal consumption in period (forced savings in currency terms).
fn sav(c: &mut Ctx) -> f64 {
    // unfilled demand=forced savings
    let mut v0 = c.v("Cd") - c.v("C");
    v0 = round_tol(v0, 0.0, 0.001); // avoid rounding errors on zero

    c.v("SavAcc"); // ensure up-to-date before
    c.incr("SavAcc", v0); // updating accumulated

    v0
}

/// Government tax income.
fn tax(c: &mut Ctx) -> f64 {
    let cap = capsecl0(c);
    let con = consecl0(c);
    let lab = labsupl0(c);
    let x = c.vs(cap, "Tax1") + c.vs(con, "Tax2") + c.vs(lab, "TaxW") + c.v("TaxDiv");
    x
}

/// Total taxes paid on dividends.
fn tax_div(c: &mut Ctx) -> f64 {
    if c.v("flagTax") >= 2.0 { c.vl("Div", 1) * c.v("tr") } else { 0.0 }
}

/// Cost (new equity) of firm entries in both industrial sectors.
/// This variable must be explicitly recalculated after entry/exit.
fn c_entry(c: &mut Ctx) -> f64 {
    let cap = capsecl0(c);
    let con = consecl0(c);
    let x = c.vs(cap, "cEntry1");
    x + c.vs(con, "cEntry2")
}

/// Credits (returned equity) from firm exits in both industrial sectors.
/// This variable must be explicitly recalculated after entry/exit.
fn c_exit(c: &mut Ctx) -> f64 {
    let cap = capsecl0(c);
    let con = consecl0(c);
    let x = c.vs(cap, "cExit1");
    x + c.vs(con, "cExit2")
}

/// Notional overall productivity (bounded) rate of change. Used for wages adjustment only.
fn d_ab(c: &mut Ctx) -> f64 {
    let lim = c.v("mLim");
    let per = c.v("mPer");
    let p = c.p;
    mov_avg_bound(c, p, "A", lim, per)
}

/// Gross domestic product (log) growth rate.
fn d_gdp(c: &mut Ctx) -> f64 {
    if c.t() > 1.0 {
        let x = c.v("GDPreal").ln();
        x - c.vl("GDPreal", 1).ln()
    } else {
        0.0
    }
}

/// Perform the entry and exit process in all sectors.
fn entry_exit(c: &mut Ctx) -> f64 {
    // ensure aggregates depending on firm objects are computed
    c.update(); // country variables
    let o = finsecl0(c);
    c.updates(o); // financial sector variables
    let o = labsupl0(c);
    c.updates(o); // labor-supply variables
    let o = macstal0(c);
    c.updates(o); // statistics-only variables
    let o = secstal0(c);
    c.updates(o);
    let o = labstal0(c);
    c.updates(o);

    let cap = capsecl0(c);
    let con = consecl0(c);
    let v0 = c.vs(cap, "entry1exit") + c.vs(con, "entry2exit");

    // recompute variables affected by entry/exit
    c.recalc("Eq"); // total equity after exit/entry
    c.recalc("cEntry"); // new equity after exit/entry
    c.recalc("cExit"); // returned equity from exits
    let fin = finsecl0(c);
    c.recalcs(fin, "BadDeb"); // bad debt update after exits

    v0
}

/// Initialize the K+S country object. It is run only once per country.
fn init_country(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 4];

    c.parameter(); // execute only once per country

    // create the new country as an extension to current 'Country' object
    // country-level pointers to speed-up the access to individual containers
    let p = c.p;
    let ext = CountryE {
        cap_sec: c.search("Capital").expect("Capital"),
        con_sec: c.search("Consumption").expect("Consumption"),
        fin_sec: c.search("Financial").expect("Financial"),
        lab_sup: c.search("Labor").expect("Labor"),
        mac_sta: c.search("Mac").expect("Mac"),
        sec_sta: c.search("Sec").expect("Sec"),
        lab_sta: c.search("Lab").expect("Lab"),
        firm2ptr: Vec::new(),
        firm2map: Default::default(),
    };
    c.add_ext(p, ext);

    // pointer shortcuts the access to individual market containers
    let cur1 = capsecl0(c);
    let cur2 = consecl0(c);
    let cur3 = finsecl0(c);
    let cur4 = labsupl0(c);

    // check unwanted extra instances (only one of each is required)
    if c.count("Capital") > 1.0
        || c.count("Consumption") > 1.0
        || c.count("Financial") > 1.0
        || c.count("Labor") > 1.0
        || c.count("Stats") > 1.0
        || c.counts(cur1, "Firm1") > 1.0
        || c.counts(cur2, "Firm2") > 1.0
        || { let f = c.searchs(cur1, "Firm1").expect("Firm1"); c.counts(f, "Cli") > 1.0 }
        || { let f = c.searchs(cur2, "Firm2").expect("Firm2"); c.counts(f, "Broch") > 1.0 }
        || { let f = c.searchs(cur2, "Firm2").expect("Firm2"); c.counts(f, "Vint") > 1.0 }
    {
        // PLOG: multiple-instance objects not allowed, aborting!
        c.abort();
    }

    // ensure initial number of firms and banks is consistent
    if c.vs(cur1, "F1") < 1.0 || c.vs(cur2, "F2") < 1.0 {
        // PLOG: invalid number of agents, aborting!
        c.abort();
    }

    // adjust parameters required to be integer and positive
    let m2v = max(1.0, c.vs(cur2, "m2").ceil());
    c.writes(cur2, "m2", m2v);

    // prepare data required to set initial conditions
    let eta = c.vs(cur2, "eta"); // technical machine life time
    let m1 = c.vs(cur1, "m1"); // output factor in sector 1
    let m2 = c.vs(cur2, "m2"); // output factor in sector 2
    let mu1 = c.vs(cur1, "mu1"); // mark-up in sector 1
    let mu20 = c.vs(cur2, "mu20"); // initial mark-up in sector 2
    let phi = c.vs(cur4, "phi"); // unemployment benefit rate
    let f1 = c.vs(cur1, "F1") as i32; // number of firms in sector 1
    let f2 = c.vs(cur2, "F2") as i32; // number of firms in sector 2
    let ls0 = c.vs(cur4, "Ls0") as i32; // initial labor supply

    let b = c.vs(cur2, "b");
    let btau0 = (1.0 + mu1) * INIPROD / (m1 * m2 * b); // initial productivity in sec. 1
    let c10 = INIWAGE / (btau0 * m1); // initial cost in sector 1
    let c20 = INIWAGE / INIPROD; // initial cost in sector 2 (integer division in C++: 1/1)
    let p10 = (1.0 + mu1) * c10; // initial price sector 1
    let p20 = (1.0 + mu20) * c20; // initial price sector 2
    let tr_w = if c.v("flagTax") > 0.0 { c.v("tr") } else { 0.0 }; // tax rate on wages
    let k0 = ls0 as f64 * INIWAGE / p20; // full employment K required
    let d10 = k0 / (m2 * eta); // initial demand for sector 1
    let rd0 = c.vs(cur1, "nu") * d10 * p10; // initial R&D expense
    let d20 = ((d10 * c10 + rd0) * (1.0 - phi - tr_w) + ls0 as f64 * INIWAGE * phi) / (mu20 + phi + tr_w) * c20; // initial demand for sector 2
    let ld10 = rd0 / INIWAGE + d10 / (btau0 * m1); // initial labor demand sec. 1
    let ld20 = d20 / INIPROD; // initial labor demand sector 2
    let a0 = (d10 * p10 + d20 * p20) / (ld10 + ld20); // initial prod.
    let eavg0 = (c.vs(cur2, "omega1") + c.vs(cur2, "omega2")) / 2.0; // initial competitiveness
    let g0 = c.v("gG") * ls0 as f64; // initial public spending

    // reserve space for country-level non-initialized vectors
    c.ext_mut::<CountryE>(p).firm2ptr.reserve(f2 as usize); // sector 2 firm objects

    // reset serial ID counters for dynamic objects
    c.writes(cur1, "lastID1", 0.0);
    c.writes(cur2, "lastID2", 0.0);

    // initialize lagged variables depending on parameters
    c.writel("A", a0, -1.0);
    c.writel("G", g0, -1.0);
    c.writes(cur1, "pK0", p10);
    c.writes(cur2, "pC0", p20);
    c.writels(cur1, "A1", btau0, -1.0);
    c.writels(cur1, "PPI", p10, -1.0);
    c.writels(cur1, "p1avg", p10, -1.0);
    c.writels(cur2, "CPI", p20, -1.0);
    c.writels(cur2, "Eavg", eavg0, -1.0);
    c.writels(cur2, "c2", c20, -1.0);
    c.writels(cur4, "Ls", ls0 as f64, -1.0);
    c.writels(cur4, "w", INIWAGE, -1.0);

    // create firms' objects and set initial values
    let f = c.searchs(cur1, "Firm1").expect("Firm1");
    c.delete(f); // remove empty firm instances
    let f = c.searchs(cur2, "Firm2").expect("Firm2");
    c.delete(f);

    v[1] = entry_firm1(c, cur1, f1, true); // add capital-good firms
    c.init_tsearchts(cur1, "Firm1", f1 as f64); // prepare turbo search indexing

    v[1] += entry_firm2(c, cur2, f2, true); // add consumer-good firms
    c.vs(cur2, "firm2maps"); // update the mapping vectors

    c.writel("Eq", v[1], -1.0); // save existing equity

    // set bank initial assets according to existing loans to firms
    v[2] = 0.0;
    v[3] = 0.0; // loans and deposits accumulator

    let mut cur5 = c.firsts(cur1, "Firm1");
    while let Some(f) = cur5 {
        v[2] += c.vls(f, "_Deb1", 1); // firm debt
        v[3] += c.vls(f, "_NW1", 1); // firm net wealth
        cur5 = c.next(f);
    }

    let mut cur6 = c.firsts(cur2, "Firm2");
    while let Some(f) = cur6 {
        v[2] += c.vls(f, "_Deb2", 1); // firm debt
        v[3] += c.vls(f, "_NW2", 1); // firm net wealth
        cur6 = c.next(f);
    }

    c.writels(cur3, "Loans", v[2], -1.0); // bank loans
    c.writels(cur3, "Depo", v[3], -1.0); // bank deposits

    1.0
}

pub fn register(r: &mut Reg) {
    // 'cEntry', 'cExit' also have EQUATION_DUMMY( .., "" ) definitions in the C++; the first definition in
    // the equation map (the real equation) wins, as std::unordered_map::insert keeps the first of duplicate keys.
    r.extend([
        ("Cd", EqDef::Fn(cd)),
        ("G", EqDef::Fn(g)),
        ("SavAcc", EqDef::Fn(sav_acc)),
        ("A", EqDef::Fn(a)),
        ("C", EqDef::Fn(cc)),
        ("Creal", EqDef::Fn(creal)),
        ("Deb", EqDef::Fn(deb)),
        ("DebGDP", EqDef::Fn(deb_gdp)),
        ("Def", EqDef::Fn(def)),
        ("DefP", EqDef::Fn(def_p)),
        ("DefPgdp", EqDef::Fn(def_p_gdp)),
        ("Div", EqDef::Fn(div)),
        ("Eq", EqDef::Fn(eq)),
        ("GDPreal", EqDef::Fn(gdp_real)),
        ("GDPnom", EqDef::Fn(gdp_nom)),
        ("Sav", EqDef::Fn(sav)),
        ("Tax", EqDef::Fn(tax)),
        ("TaxDiv", EqDef::Fn(tax_div)),
        ("cEntry", EqDef::Fn(c_entry)),
        ("cExit", EqDef::Fn(c_exit)),
        ("dAb", EqDef::Fn(d_ab)),
        ("dGDP", EqDef::Fn(d_gdp)),
        ("entryExit", EqDef::Fn(entry_exit)),
        ("initCountry", EqDef::Fn(init_country)),
    ]);
}
