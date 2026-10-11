//! fun_KS_support.h: support functions of the K+S model, plus the C/LSD numeric helpers its code uses.
use crate::model::{Ctx, ObjId};

use super::class::*;

// ---------------------------------------------------------------------------------------------- C / LSD numerics

/// LSD's `abs` macro, `_abs( double )`: note `-1 * 0.0` is `-0.0`.
pub fn abs(a: f64) -> f64 {
    if a > 0.0 { a } else { -1.0 * a }
}

/// LSD's `max( double, double )` (util.cpp): returns `b` unless `a > b`.
pub fn max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

/// LSD's `min( double, double )` (util.cpp): returns `b` unless `a < b`.
pub fn min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

/// Macro ROUND( V, Ref, Tol ): round values too close to a reference.
pub fn round_tol(v: f64, r: f64, tol: f64) -> f64 {
    if abs(v - r) > tol { v } else { r }
}

/// The K+S macro NEW_VS: compute a variable (the C++ only logs when it was already computed).
pub fn new_vs(c: &mut Ctx, o: ObjId, lab: &str) -> f64 {
    c.last_calcs(o, lab);
    c.vs(o, lab)
}

// ---------------------------------------------------------------------------------------------- general support

/// Calculate the bounded, moving-average growth rate of variable (if `lim` is zero, there is no bounding).
pub fn mov_avg_bound(c: &mut Ctx, obj: ObjId, var: &str, lim: f64, per: f64) -> f64 {
    let mut sum_g = 0.0;
    let mut i: i32 = 0;
    while (i as f64) < per {
        if c.m.t - i <= 0 {
            break; // just go to t=0
        }
        let prev = c.vls(obj, var, i + 1);
        let mut g = if prev != 0.0 { c.vls(obj, var, i) / prev - 1.0 } else { 0.0 };
        if lim > 0.0 {
            g = max(min(g, lim), -lim); // apply bounds
        }
        sum_g += g;
        i += 1;
    }
    sum_g / i as f64
}

/// check_error: the C++ only logs the message; the counter is what matters.
pub fn check_error(cond: bool, errors: &mut i32) {
    if cond {
        *errors += 1;
    }
}

// ---------------------------------------------------------------------------------------------- financial support

const CD_VAR: [&str; 2] = ["_CD1", "_CD2"];
const CDC_VAR: [&str; 2] = ["_CD1c", "_CD2c"];
const CS_VAR: [&str; 2] = ["_CS1", "_CS2"];
const DEB_VAR: [&str; 2] = ["_Deb1", "_Deb2"];
const NW_VAR: [&str; 2] = ["_NW1", "_NW2"];

fn sector_of(c: &Ctx, firm: ObjId) -> usize {
    if c.names(firm) == "Firm1" { 0 } else { 1 }
}

/// Update firm debt in equations '_Q1', '_Tax1', '_Q2', '_EI', '_SI', '_Tax2'.
pub fn update_debt(c: &mut Ctx, firm: ObjId, desired: f64, loan: f64) -> f64 {
    let sec = sector_of(c, firm);
    if desired > 0.0 {
        // ignore loan repayment
        c.incrs(firm, CD_VAR[sec], desired); // desired credit
        c.incrs(firm, CDC_VAR[sec], desired - loan); // credit constraint
        c.incrs(firm, CS_VAR[sec], loan); // supplied credit
    }
    let mut deb = c.vs(firm, DEB_VAR[sec]);
    // take new loan/repay debt from/to bank
    if loan != 0.0 {
        deb = c.incrs(firm, DEB_VAR[sec], loan);
    }
    deb
}

/// Update firm deposits in equations '_Q1', '_Tax1', '_Q2', '_EI', '_SI', '_Tax2'.
pub fn update_depo(c: &mut Ctx, firm: ObjId, depo: f64, incr: bool) -> f64 {
    let sec = sector_of(c, firm);
    // update total firm net worth (deposits)
    if incr {
        let mut nw = c.vs(firm, NW_VAR[sec]);
        if depo != 0.0 {
            nw = c.incrs(firm, NW_VAR[sec], depo);
        }
        nw
    } else {
        c.writes(firm, NW_VAR[sec], depo)
    }
}

const CI_VAR: [&str; 2] = ["", "_CI"];
const CSA_VAR: [&str; 2] = ["_CS1a", "_CS2a"];
const DIV_VAR: [&str; 2] = ["_Div1", "_Div2"];
const NWP_VAR: [&str; 2] = ["_NW1p", "_NW2p"];

/// Manage firm cash flow in equations '_Tax1', '_Tax2'.
pub fn cash_flow(c: &mut Ctx, firm: ObjId, profit: f64, tax: f64) -> f64 {
    let sec = match c.names(firm) {
        "Firm1" => 0,
        "Firm2" => 1,
        _ => 2,
    };
    let dividends = c.vls(firm, DIV_VAR[sec], 1); // shareholder dividends
    let cash_free = profit - tax - dividends; // final free cash flow

    if sec > 0 {
        c.vs(firm, CI_VAR[sec]); // ensure canc. invest. reimbursed
    }

    let provision = if sec < 2 { c.vs(firm, NWP_VAR[sec]) } else { 0.0 }; // prod. cost
    let depo = update_depo(c, firm, provision, true); // current bank deposits

    if cash_free < 0.0 {
        // must finance losses?
        if depo >= -cash_free {
            // deposits cover losses?
            update_depo(c, firm, cash_free, true); // draw from deposits
        } else {
            let cred_avb = c.vs(firm, CSA_VAR[sec]); // available credit
            let cred_des = -cash_free - depo; // desired credit

            update_debt(c, firm, cred_des, cred_des); // finance all in any case

            if cred_avb >= cred_des {
                // could finance losses?
                update_depo(c, firm, 0.0, false); // keep going with zero deposits
            } else {
                update_depo(c, firm, -1e-6, false); // let negative NW (bankruptcy)
            }
        }
    } else {
        // pay debt with available cash
        let repay_des = c.vs(firm, DEB_VAR[sec]); // desired debt repayment
        if repay_des > 0.0 {
            // something to repay?
            if cash_free > repay_des {
                // can repay desired and more
                update_debt(c, firm, 0.0, -repay_des); // repay up to desired
                update_depo(c, firm, cash_free - repay_des, true); // keep the rest
            } else {
                update_debt(c, firm, 0.0, -cash_free); // repay what is possible
            }
        } else {
            update_depo(c, firm, cash_free, true); // just keep all
        }
    }

    cash_free
}

// ---------------------------------------------------------------------------------------------- capital management

/// Send machine brochure to consumption-good client firm in equations '_NC', '_supplier'.
pub fn send_brochure(c: &mut Ctx, suppl: ObjId, client: ObjId) -> ObjId {
    let cli = c.addobjs(suppl, "Cli"); // add object to new client
    let id = c.vs(client, "_ID2");
    c.writes(cli, "__IDc", id); // update client ID
    let t = c.t();
    c.writes(cli, "__tSel", t); // update selection time

    let broch = c.addobjs(client, "Broch"); // add brochure to client
    let id = c.vs(suppl, "_ID1");
    c.writes(broch, "__IDs", id); // update supplier ID
    c.write_shooks(broch, cli); // pointer to supplier client list
    c.write_shooks(cli, broch); // pointer to client brochure list

    broch
}

/// Set initial supplier for entrant in equations 'entry2exit'.
pub fn set_supplier(c: &mut Ctx, firm: ObjId) -> ObjId {
    let gp = c.grandparents(firm);
    let cap = country_e(c, gp).cap_sec;

    let suppl = c.rnddrawls(cap, "Firm1", "_Atau", 0).expect("supplier"); // draw capital supplier
    let broch = send_brochure(c, suppl, firm); // get supplier brochure
    c.write_hooks(firm, SUPPL, broch); // pointer to current supplier
    c.incrs(suppl, "_NC", 1.0); // update supplier's clients #

    suppl
}

/// The Cli entry of the firm on its current supplier's client list: SHOOKS( HOOKS( firm, SUPPL ) ).
fn supplier_cli(c: &Ctx, firm: ObjId) -> ObjId {
    c.shooks(c.hooks(firm, SUPPL).expect("supplier brochure")).expect("client entry")
}

/// Send new machine order in equations '_EI', '_SI'.
pub fn send_order(c: &mut Ctx, firm: ObjId, n_mach: f64) {
    // find firm entry on supplier client list
    let cli = supplier_cli(c, firm);

    let t = c.t();
    if c.vs(cli, "__tOrd") < t {
        // if first order in period
        c.writes(cli, "__nOrd", n_mach); // set new order size
        c.writes(cli, "__tOrd", t); // set order time
        c.writes(cli, "__nCan", 0.0); // no machine canceled yet
    } else {
        c.incrs(cli, "__nOrd", n_mach); // increase existing order size
    }
}

/// Perform investment according to available funding in equations '_EI', '_SI'.
pub fn invest(c: &mut Ctx, firm: ObjId, desired: f64) -> f64 {
    if desired <= 0.0 {
        return 0.0;
    }

    let par = c.up(firm);
    let m2 = c.vs(par, "m2"); // machine output per period
    let cs2a = c.vs(firm, "_CS2a"); // available credit supply
    let mut nw2 = c.vs(firm, "_NW2"); // net worth (cash available)
    let suppl = c.up(supplier_cli(c, firm));
    let p1 = c.vs(suppl, "_p1");

    let mut inv_cost = p1 * desired / m2; // desired investment cost
    let inv;

    if inv_cost <= nw2 {
        // can invest with own funds?
        inv = desired; // invest as planned
        nw2 -= inv_cost; // remove machines cost from cash
    } else {
        let (loan, loan_des);
        if inv_cost <= nw2 + cs2a {
            // possible to finance all?
            inv = desired; // invest as planned
            loan = inv_cost - nw2; // finance the difference
            loan_des = loan;
            nw2 = 0.0; // no cash
        } else {
            // credit constrained firm
            // invest as much as the available finance allows, rounded # machines
            inv = max(((nw2 + cs2a) / p1).floor() * m2, 0.0);
            loan_des = inv_cost - nw2; // desired credit

            if inv == 0.0 {
                loan = 0.0; // no finance
            } else {
                inv_cost = p1 * inv / m2; // reduced investment cost
                if inv_cost <= nw2 {
                    // just own funds?
                    loan = 0.0;
                    nw2 -= inv_cost; // remove machines cost from cash
                } else {
                    loan = inv_cost - nw2; // finance the difference
                    nw2 = 0.0; // no cash
                }
            }
        }

        update_debt(c, firm, loan_des, loan); // update debt (desired/granted)
    }

    if inv > 0.0 {
        update_depo(c, firm, nw2, false); // update the firm net worth
        send_order(c, firm, (inv / m2).round()); // order to machine supplier
    }

    inv
}

/// Add new vintage to the capital stock of a firm in equation 'K' and 'initCountry'.
pub fn add_vintage(c: &mut Ctx, firm: ObjId, n_mach: f64, new_ind: bool) {
    let suppl = c.up(supplier_cli(c, firm)); // current supplier
    let mut nmach: i32 = n_mach.floor() as i32; // integer number of machines
    let (mut age_vint, mut n_vint, a_vint, p_vint): (i32, i32, f64, f64);
    let (mut cap, mut _cons) = (0, 0);

    // at t=1 firms have a mix of machines: old to new, many suppliers
    if new_ind {
        let gp = c.grandparents(firm);
        cap = country_e(c, gp).cap_sec;
        _cons = country_e(c, gp).con_sec;

        age_vint = (c.vs(_cons, "eta") + 1.0) as i32; // age of oldest machine
        n_vint = (n_mach / age_vint as f64).ceil() as i32; // machines per vintage
        a_vint = INIPROD; // initial product. in sector 2
        p_vint = c.vls(cap, "p1avg", 1); // initial machine price
    } else {
        age_vint = (1.0 - c.t()) as i32;
        n_vint = nmach;
        a_vint = c.vs(suppl, "_Atau");
        p_vint = c.vs(suppl, "_p1");
    }

    while nmach > 0 {
        let (cur, vint);
        if new_ind {
            cur = c.rnddraw_fairs(cap, "Firm1").expect("supplier"); // draw another supplier
            if cur == suppl {
                continue; // don't use current supplier
            }
            let tu = (c.t() - 1.0) as i32;
            vint = c.addobjls(firm, "Vint", tu); // recalculate in t=1
        } else {
            cur = suppl; // just use current supplier
            vint = c.addobjs(firm, "Vint"); // just recalculate in next t
        }

        let top = c.hooks(firm, TOPVINT);
        c.write_shooks(vint, top); // save previous vintage
        c.write_hooks(firm, TOPVINT, vint); // save pointer to top vintage

        let id1 = c.vs(cur, "_ID1");
        let t = c.t();
        c.writes(vint, "__IDvint", vnt(t, id1) as f64); // vintage ID
        c.writes(vint, "__Avint", a_vint); // vintage productivity
        c.writes(vint, "__nVint", n_vint as f64); // number of machines in vintage
        c.writes(vint, "__pVint", p_vint); // price of machines in vintage
        c.writes(vint, "__tVint", (1 - age_vint) as f64); // vintage build time

        nmach -= n_vint;
        age_vint -= 1;

        if age_vint > 0 && nmach % age_vint == 0 {
            // exact ratio missing?
            n_vint = nmach / age_vint; // adjust machines per vintage
        }
    }
}

/// Scrap (remove) vintage from capital stock in equation 'K'.
/// Return -1 if last vintage (not removed but shrank to 1 machine).
pub fn scrap_vintage(c: &mut Ctx, vint: ObjId) -> f64 {
    let rs;
    if let Some(next) = c.m.next(vint) {
        // don't remove last vintage
        // remove as previous vintage from next vintage
        if c.shooks(next) == Some(vint) {
            c.write_shooks(next, None::<ObjId>);
        }
        rs = abs(c.vs(vint, "__RSvint"));
        c.delete(vint); // delete vintage
    } else {
        rs = -1.0; // signal last machine
        c.writes(vint, "__nVint", 1.0); // keep just 1 machine
    }
    rs
}

// ---------------------------------------------------------------------------------------------- firm entry-exit

/// Add and configure entrant capital-good firm object(s) and required hooks in equations 'entry1exit' and 'initCountry'.
pub fn entry_firm1(c: &mut Ctx, sector: ObjId, mut n: i32, new_ind: bool) -> f64 {
    let (mut atau, mut btau, d10, mut deb1_tot, mut eq1_tot, mut nw1_tot): (f64, f64, f64, f64, f64, f64);
    let (nw10, f1, t1ent): (f64, f64, i32);
    let (atau_max, btau_max): (f64, f64);
    let par = c.up(sector);
    let cons = country_e(c, par).con_sec;
    let lab = country_e(c, par).lab_sup;

    let deb10ratio = c.vs(sector, "Deb10ratio"); // bank fin. to equity ratio
    let phi3 = c.vs(sector, "Phi3"); // lower support for wealth share
    let phi4 = c.vs(sector, "Phi4"); // upper support for wealth share
    let alpha2 = c.vs(sector, "alpha2"); // lower support for imitation
    let beta2 = c.vs(sector, "beta2"); // upper support for imitation
    let mu1 = c.vs(sector, "mu1"); // mark-up in sector 1
    let m1 = c.vs(sector, "m1"); // worker production scale
    let _nu = c.vs(sector, "nu"); // share of R&D expenses
    let x5 = c.vs(sector, "x5"); // entrant upper advantage
    let w = c.vs(lab, "w"); // current wage

    if new_ind {
        let f2 = c.vs(cons, "F2");
        let m2 = c.vs(cons, "m2"); // machine output per period

        atau = INIPROD;
        atau_max = INIPROD; // initial productivities to use
        let b = c.vs(cons, "b");
        btau = (1.0 + mu1) * atau / (m1 * m2 * b);
        btau_max = btau; // and build machines (s. s.)
        nw10 = c.vs(sector, "NW10"); // initial wealth in sector 1
        f1 = 1.0 / n as f64; // fair share
        t1ent = 0; // entered before t=1

        // initial demand expectation, assuming all sector 2 firms, 1/eta
        // replacement factor and fair share in sector 1 and full employment
        let p20 = c.vls(cons, "CPI", 1);
        let k0 = (c.vs(lab, "Ls0") * INIWAGE / p20 / f2 / m2).ceil() * m2;

        d10 = f2 * k0 / m2 / c.vs(cons, "eta") / n as f64;
    } else {
        // function call max( .., ..): GCC evaluates the arguments right to left
        let a2 = c.vs(sector, "NW10") * c.vs(sector, "PPI") / c.vs(sector, "pK0");
        let a1 = c.whtaves(sector, "_NW1", "_f1");
        nw10 = max(a1, a2);
        f1 = 0.0; // no market share
        t1ent = c.t() as i32; // entered now
        atau_max = c.maxs(sector, "_Atau"); // best machine productivity
        btau_max = c.maxs(sector, "_Btau"); // best productivity in sector 1

        // initial demand equal to 1 machine per client under fair share entry
        d10 = c.vs(cons, "F2") / c.vs(sector, "F1");
        atau = 0.0;
        btau = 0.0;
    }

    // add entrant firms (end of period, don't try to sell)
    deb1_tot = 0.0;
    eq1_tot = 0.0;
    nw1_tot = 0.0;
    while n > 0 {
        // create object, only recalculate in t if new industry
        let firm = if new_ind {
            let tu = (c.t() - 1.0) as i32;
            c.addobjls(sector, "Firm1", tu)
        } else {
            c.addobjs(sector, "Firm1")
        };

        let id1 = c.incrs(sector, "lastID1", 1.0) as i32; // new firm ID
        c.writes(firm, "_ID1", id1 as f64);

        let cli = c.searchs(firm, "Cli").expect("Cli instance");
        c.delete(cli); // remove empty instances

        if !new_ind {
            // initial labor productivity (imitation from best firm)
            atau = c.beta(alpha2, beta2); // draw A from Beta(alpha,beta)
            atau = atau * (atau_max * (1.0 + x5)); // fraction of top firm
            btau = c.beta(alpha2, beta2); // draw B from Beta(alpha,beta)
            btau = btau * (btau_max * (1.0 + x5)); // fraction of top firm
        }

        // initial cost, price and net wealth
        let mult = if new_ind { 1.0 } else { c.uniform(phi3, phi4) }; // NW multiple
        let c1 = w / (btau * m1); // unit cost
        let p1 = (1.0 + mu1) * c1; // unit price
        let rd0 = _nu * d10 * p1; // R&D expense
        let l1rd = rd0 / w; // workers in R&D

        // accumulate capital costs
        let nw1 = mult * nw10;
        nw1_tot += nw1;
        let deb1 = nw1 * deb10ratio;
        deb1_tot += deb1;
        let eq1 = nw1 * (1.0 - deb10ratio);
        eq1_tot += eq1;

        // initialize variables
        c.writes(firm, "_Eq1", eq1);
        c.writes(firm, "_t1ent", t1ent as f64);
        c.writells(firm, "_Atau", atau, t1ent, 1);
        c.writells(firm, "_Btau", btau, t1ent, 1);
        c.writells(firm, "_f1", f1, t1ent, 1);
        c.writells(firm, "_p1", p1, t1ent, 1);

        if new_ind {
            c.writells(firm, "_Deb1", deb1, t1ent, 1);
            c.writells(firm, "_L1rd", l1rd, t1ent, 1);
            c.writells(firm, "_NW1", nw1, t1ent, 1);
            c.writells(firm, "_RD", rd0, t1ent, 1);
            c.writells(firm, "_cTau", w / atau, t1ent, 1);
        } else {
            c.writes(firm, "_Atau", atau);
            c.writes(firm, "_Btau", btau);
            c.writes(firm, "_Deb1", deb1);
            c.writes(firm, "_L1rd", l1rd);
            c.writes(firm, "_NW1", nw1);
            c.writes(firm, "_RD", rd0);
            c.writes(firm, "_c1", c1);
            c.writes(firm, "_cTau", w / atau);
            c.writes(firm, "_p1", p1);

            // compute variables requiring calculation in t
            c.recalcs(firm, "_Deb1max"); // prudential credit limit
            c.recalcs(firm, "_NC"); // set initial clients
            c.vs(firm, "_CS1a"); // update credit supply
        }
        n -= 1;
    }

    if new_ind {
        // set t=0 values
        c.writells(sector, "Deb1", deb1_tot, t1ent, 1);
        c.writells(sector, "Eq1", eq1_tot, t1ent, 1);
        c.writells(sector, "NW1", nw1_tot, t1ent, 1);
    } else {
        // just account new equity
        c.incrs(sector, "Eq1", eq1_tot);
        c.incrs(sector, "cEntry1", eq1_tot);
    }

    eq1_tot
}

/// Add and configure entrant consumer-good firm object(s) and required hooks in equations 'entry2exit' and 'initCountry'.
pub fn entry_firm2(c: &mut Ctx, sector: ObjId, mut n: i32, new_ind: bool) -> f64 {
    let par = c.up(sector);
    let cap = country_e(c, par).cap_sec;
    let lab = country_e(c, par).lab_sup;

    let deb20ratio = c.vs(sector, "Deb20ratio"); // bank fin. to equity ratio
    let phi1 = c.vs(sector, "Phi1"); // lower support for K share
    let phi2 = c.vs(sector, "Phi2"); // upper support for K share
    let iota = c.vs(sector, "iota"); // desired inventories factor
    let mu20 = c.vs(sector, "mu20"); // initial mark-up in sector 2
    let m2 = c.vs(sector, "m2"); // machine output per period
    let p10 = c.vls(cap, "p1avg", 1); // initial machine price
    let u = c.vs(sector, "u"); // desired capital utilization
    let w = c.vs(lab, "w"); // current wage

    let (d20, e, mut k_, n_, nw20, q2u, f2, life2cycle): (f64, f64, f64, f64, f64, f64, f64, f64);
    let t2ent: i32;
    if new_ind {
        let phi = c.vs(lab, "phi"); // unemployment benefit rate
        let cap_mu1 = c.vs(cap, "mu1");
        let c10 = p10 / (1.0 + cap_mu1); // initial unit cost sec. 1
        let c20 = INIWAGE / INIPROD; // initial unit cost sec. 2
        let p20 = (1.0 + mu20) * c20; // initial consumer-good price
        let tax = c.vs(par, "flagTax");
        let tr_w = if tax > 0.0 { c.vs(par, "tr") } else { 0.0 }; // tax rate on wages
        let k0 = (c.vs(lab, "Ls0") * INIWAGE / p20 / n as f64 / m2).ceil() * m2; // full employment K required
        let sir0 = n as f64 * k0 / m2 / c.vs(sector, "eta"); // substit. real invest.
        let rd0 = c.vs(cap, "nu") * sir0 * p10; // initial R&D expense

        // initial steady state demand under fair share
        d20 = ((sir0 * c10 + rd0) * (1.0 - phi - tr_w) + c.vs(lab, "Ls0") * INIWAGE * phi) / (mu20 + phi + tr_w) * c20 / n as f64;
        e = c.vls(sector, "Eavg", 1); // initial competitiveness
        k_ = k0; // initial capital in sector 2
        n_ = iota * d20; // initial inventories
        nw20 = c.vs(sector, "NW20"); // initial wealth in sector 2
        q2u = 1.0; // initial capacity utilization
        f2 = 1.0 / n as f64; // fair share
        life2cycle = 1.0; // start as incumbent
        t2ent = 0; // entered before t=1
    } else {
        d20 = 0.0;
        e = c.vs(sector, "Eavg"); // average competitiveness
        k_ = c.whtaves(sector, "_K", "_f2"); // w. avg. capital in sector 2
        n_ = 0.0; // inventories
        nw20 = c.whtaves(sector, "_NW2", "_f2"); // average wealth in sector 2
        q2u = c.vs(sector, "Q2u"); // capacity utilization
        f2 = 0.0; // no market share
        life2cycle = 0.0; // start as pre-operat. entrant
        t2ent = c.t() as i32; // entered now
    }

    // add entrant firms (end of period, don't try to sell)
    let (mut deb2_tot, mut eq2_tot, mut nw2_tot, mut k_tot, mut n_tot) = (0.0, 0.0, 0.0, 0.0, 0.0);
    while n > 0 {
        // create object, only recalculate in t if new industry
        let firm = if new_ind {
            let tu = (c.t() - 1.0) as i32;
            c.addobjls(sector, "Firm2", tu)
        } else {
            c.addobjs(sector, "Firm2")
        };

        let id2 = c.incrs(sector, "lastID2", 1.0) as i32; // new firm ID
        c.writes(firm, "_ID2", id2 as f64);

        c.addhooks(firm, FIRM2HK); // add object hooks
        let vint = c.searchs(firm, "Vint").expect("Vint instance");
        c.delete(vint); // remove empty instances
        let broch = c.searchs(firm, "Broch").expect("Broch instance");
        c.delete(broch);

        // select initial machine supplier
        let suppl = set_supplier(c, firm);

        // initial desired capital/expected demand, rounded to # of machines
        let mut mult = if new_ind { 1.0 } else { c.uniform(phi1, phi2) }; // capital multiple
        k_ = (max(mult * k_ / m2, 1.0)).ceil() * m2;
        k_tot += k_;
        let d2e = if new_ind { d20 } else { u * k_ };
        n_tot += n_;

        // define entrant initial free cash (1 period wages or default minimum)
        mult = if new_ind { 1.0 } else { c.uniform(phi1, phi2) }; // NW multiple
        let a2 = c.vs(suppl, "_Atau"); // initial productivity
        let c2 = w / a2; // initial unit costs
        let p2 = (1.0 + mu20) * c2; // initial price
        let mut nw2f = (1.0 + iota) * d2e * c2; // initial free cash
        nw2f = max(nw2f, mult * nw20);

        // initial equity must pay initial capital and wages
        let nw2 = if new_ind { nw2f } else { c.vs(suppl, "_p1") * k_ / m2 + nw2f };
        nw2_tot += nw2;
        let deb2 = nw2 * deb20ratio;
        deb2_tot += deb2;
        let eq2 = nw2 * (1.0 - deb20ratio); // accumulated equity (all firms)
        eq2_tot += eq2;

        // initialize variables
        c.writes(firm, "_Eq2", eq2);
        c.writes(firm, "_t2ent", t2ent as f64);
        c.writes(firm, "_life2cycle", life2cycle);
        c.writells(firm, "_A2", a2, t2ent, 1);
        c.writells(firm, "_f2", f2, t2ent, 1);
        c.writells(firm, "_f2", f2, t2ent, 2);
        c.writells(firm, "_mu2", mu20, t2ent, 1);
        c.writells(firm, "_p2", p2, t2ent, 1);

        for i in 1..=4 {
            c.writells(firm, "_D2", d2e, t2ent, i);
            c.writells(firm, "_D2d", d2e, t2ent, i);
        }

        if new_ind {
            c.writells(firm, "_Deb2", deb2, t2ent, 1);
            c.writells(firm, "_K", k_, t2ent, 1);
            c.writells(firm, "_N", n_, t2ent, 1);
            c.writells(firm, "_NW2", nw2, t2ent, 1);

            add_vintage(c, firm, k_ / m2, new_ind); // first machine vintages
        } else {
            c.writes(firm, "_A2", a2);
            c.writes(firm, "_D2e", d2e);
            c.writes(firm, "_Deb2", deb2);
            c.writes(firm, "_E", e);
            c.writes(firm, "_Kd", k_);
            c.writes(firm, "_NW2", nw2);
            c.writes(firm, "_Q2u", q2u);
            c.writes(firm, "_c2", c2);
            c.writes(firm, "_c2e", c2);
            c.writes(firm, "_mu2", mu20);
            c.writes(firm, "_p2", p2);

            // compute variables requiring calculation in t
            c.recalcs(firm, "_Deb2max"); // prudential credit limit
            c.vs(firm, "_CS2a"); // update credit supply
        }
        n -= 1;
    }

    if new_ind {
        // set t=0 values
        c.writells(sector, "Deb2", deb2_tot, t2ent, 1);
        c.writells(sector, "Eq2", eq2_tot, t2ent, 1);
        c.writells(sector, "K", k_tot, t2ent, 1);
        c.writells(sector, "N", n_tot, t2ent, 1);
        c.writells(sector, "NW2", nw2_tot, t2ent, 1);
    } else {
        // just account new equity
        c.incrs(sector, "Eq2", eq2_tot);
        c.incrs(sector, "cEntry2", eq2_tot);
    }

    eq2_tot
}

const BAD_DEB_VAR: [&str; 2] = ["BadDeb1", "BadDeb2"];
const EQ_VAR_FIRM: [&str; 2] = ["_Eq1", "_Eq2"];
const EQ_VAR: [&str; 2] = ["Eq1", "Eq2"];
const CEXIT_VAR: [&str; 2] = ["cExit1", "cExit2"];
const CLI_BROCH_OBJ: [&str; 2] = ["Cli", "Broch"];

/// Remove firm object and existing hooks in equation 'entry1exit', 'entry2exit'.
pub fn exit_firm(c: &mut Ctx, firm: ObjId) -> f64 {
    let gp = c.grandparents(firm);
    let fin = c.searchs(gp, "Financial").expect("Financial");
    let sec = sector_of(c, firm);
    let par = c.up(firm);

    // remove equity from sector total
    let eq = c.vs(firm, EQ_VAR_FIRM[sec]);
    c.incrs(par, EQ_VAR[sec], -eq);

    // account liquidation equity credit of shareholder or bad debt cost of bank
    let nw = c.vs(firm, NW_VAR[sec]);
    let deb = c.vs(firm, DEB_VAR[sec]);
    let liq_val = nw - deb;
    let liq_eq;

    if liq_val < 0.0 {
        // account bank losses, if any
        liq_eq = 0.0; // no liquidation equity
        c.vs(fin, BAD_DEB_VAR[sec]); // ensure reset in t
        c.incrs(fin, BAD_DEB_VAR[sec], -liq_val); // accumulate bank losses
    } else {
        liq_eq = round_tol(liq_val, 0.0, 0.01); // no liquidation equity credit
        c.incrs(par, CEXIT_VAR[sec], liq_eq);
    }

    // leave counterpart lists
    let mut cur = c.firsts(firm, CLI_BROCH_OBJ[sec]);
    while let Some(cli) = cur {
        let hk = c.shooks(cli).expect("counterpart entry");
        c.delete(hk); // delete from counterpart list
        cur = c.next(cli);
    }

    if sec == 1 {
        // update firm map before removing LSD object in consumption sector
        let id = c.vs(firm, "_ID2") as i32;
        let gp = c.grandparents(firm);
        c.ext_mut::<CountryE>(gp).firm2map.remove(&id);
    }

    c.delete(firm);

    liq_eq
}
