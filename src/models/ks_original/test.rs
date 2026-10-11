//! fun_KS_test.h: test-only equations (consistency checks and debugging output).
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;

use crate::model::{Ctx, EqDef, ObjId};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

const TEST1FILE: &str = "firms1.csv"; // names of test files on disk
const TEST2FILE: &str = "firms2.csv";

const TOL: f64 = 0.1; // general error tolerance
const SFCTHRD: f64 = 1e-4; // threshold for SFC detection

/// Test the balances of the balance-sheet and the transaction-flow matrices. Based on Nikiforos & Zezza 2017.
fn test_sfc(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 7];
    let par = c.parent();
    let (cap, con, fin, lab) = (capsecl1(c), consecl1(c), finsecl1(c), labsupl1(c));

    c.vs(par, "entryExit"); // ensure period is settled

    // matrices data - capital-good sector
    let deb1 = c.vs(cap, "Deb1"); // bank debt/loans
    let div1_1 = c.vls(cap, "Div1", 1); // dividends
    let eq1 = c.vs(cap, "Eq1"); // equity
    let nw1 = c.vs(cap, "NW1"); // bank deposits
    let s1 = c.vs(cap, "S1"); // sales
    let tax1 = c.vs(cap, "Tax1"); // taxes
    let w1 = c.vs(cap, "W1"); // wages
    let c_entry1_1 = c.vls(cap, "cEntry1", 1); // new equity
    let c_exit1_1 = c.vls(cap, "cExit1", 1); // liquidation payouts
    let d_deb1 = deb1 - c.vls(cap, "Deb1", 1); // change in debt/loans
    let d_nw1 = nw1 - c.vls(cap, "NW1", 1); // change in deposits
    let i1 = c.vs(cap, "i1"); // interest paid on debt
    let id1 = c.vs(cap, "iD1"); // interest got on deposits
    let net_pi1 = c.vs(cap, "Pi1") - tax1; // net profits

    // matrices data - consumption-good sector
    let deb2 = c.vs(con, "Deb2"); // bank debt/loans
    let div2_1 = c.vls(con, "Div2", 1); // dividends
    let eq2 = c.vs(con, "Eq2"); // equity
    let inom = c.vs(con, "Inom"); // investment (nominal terms)
    let knom = c.vs(con, "Knom"); // capital (nominal terms)
    let nw2 = c.vs(con, "NW2"); // bank deposits
    let s2 = c.vs(con, "S2"); // sales
    let tax2 = c.vs(con, "Tax2"); // taxes
    let w2 = c.vs(con, "W2"); // wages
    let c_entry2_1 = c.vls(con, "cEntry2", 1); // new equity
    let c_exit2_1 = c.vls(con, "cExit2", 1); // liquidation payouts
    let d_deb2 = deb2 - c.vls(con, "Deb2", 1); // change in debt/loans
    let d_nw2 = nw2 - c.vls(con, "NW2", 1); // change in deposits
    let i2 = c.vs(con, "i2"); // interest paid on debt
    let id2 = c.vs(con, "iD2"); // interest got on deposits
    let net_pi2 = c.vs(con, "Pi2") - tax2; // net profits

    // matrices data - financial sector
    let bad_deb_1 = c.vls(fin, "BadDeb", 1); // bank bad debt
    let bad_deb1_1 = c.vls(fin, "BadDeb1", 1); // bank bad debt in sector 1
    let bad_deb2_1 = c.vls(fin, "BadDeb2", 1); // bank bad debt in sector 2
    let depo = c.vs(fin, "Depo"); // bank deposits
    let depo_g = c.vs(fin, "DepoG"); // government deposits at c.b.
    let depo_g_1 = c.vls(fin, "DepoG", 1);
    let loans = c.vs(fin, "Loans"); // bank loans
    let d_depo = depo - c.vls(fin, "Depo", 1); // change in deposits
    let d_depo_g = depo_g - depo_g_1; // change in gov. deposits c.b.
    let d_loans = loans - c.vls(fin, "Loans", 1); // change in loans
    let d_mb = d_loans - d_depo; // change in monetary base
    let i_b = c.vs(fin, "iB"); // interest received on loans
    let i_db = c.vs(fin, "iDb"); // interest paid on deposits
    let pi_b = c.vs(fin, "PiB"); // profits
    let r = c.vs(fin, "r"); // interest rate

    // matrices data - workers/households
    let div_1 = c.vls(par, "Div", 1); // total dividends
    let eq = c.vs(par, "Eq"); // total equity
    let sav_acc = c.vs(par, "SavAcc"); // bank deposits (= acc. savings)
    let sav_acc_1 = c.vls(par, "SavAcc", 1);
    let tax_div = c.vs(par, "TaxDiv"); // taxes on dividends
    let tax_w = c.vs(lab, "TaxW"); // taxes on wages
    let w = c.vs(lab, "W"); // total wages
    let c_entry_1 = c.vls(par, "cEntry", 1); // total new equity
    let c_exit_1 = c.vls(par, "cExit", 1); // total liquidation payouts
    let d_sav_acc = sav_acc - sav_acc_1; // change in deposits

    // matrices data matrices - country/government
    let cc = c.vs(par, "C"); // consumption
    let deb = c.vs(par, "Deb"); // government debt (total bonds)
    let deb_1 = c.vls(par, "Deb", 1);
    let g = c.vs(par, "G"); // government expenditure
    let tax = c.vs(par, "Tax"); // government income (taxes)
    let d_deb = deb - deb_1; // change in debt (bond) stock

    // balance-sheet row sums (accounting identities)
    let equities = eq - eq1 - eq2;
    let deposits = sav_acc + nw1 + nw2 - depo;
    let bank_loans = -deb1 - deb2 + loans;
    let gov_debt = deb - deb;
    let gov_depo = -depo_g + depo_g;

    v[1] = abs(equities) + abs(deposits) + abs(bank_loans) + abs(gov_debt) + abs(gov_depo);

    // balance-sheet column sums (balance/net worth)
    let bal = sav_acc + eq;
    let bal1 = nw1 - deb1 - eq1;
    let bal2 = knom + nw2 - deb2 - eq2;
    let bal_b = -depo + loans + deb - depo_g;
    let bal_g = -deb + depo_g;

    v[2] = -bal - bal1 - bal2 - bal_b - bal_g + knom;

    // transaction-flow matrix row sums (accounting identities)
    let consumption = -cc + s2;
    let investment = s1 - inom;
    let gov_expend = g - g;
    let wages = w - w1 - w2;
    let taxes = -tax_w - tax_div - tax1 - tax2 + tax;
    let profits = -net_pi1 + net_pi1 - net_pi2 + net_pi2 - pi_b + pi_b;
    let dividends = div_1 - div1_1 - div2_1;
    let new_equity = -c_entry_1 + c_entry1_1 + c_entry2_1;
    let liq_equity = c_exit_1 - c_exit1_1 - c_exit2_1;
    let bad_debt = bad_deb1_1 + bad_deb2_1 - bad_deb_1;

    let depo_intrst = r * sav_acc_1 + id1 + id2 - i_db;
    let loan_intrst = -i1 - i2 + i_b;

    let depo_chg = -d_sav_acc - d_nw1 - d_nw2 + d_depo;
    let loan_chg = d_deb1 + d_deb2 - d_loans;
    let gov_debt_chg = -d_deb + d_deb;
    let gov_depo_chg = d_depo_g - d_depo_g;

    v[3] = abs(consumption) + abs(investment) + abs(gov_expend) + abs(wages) + abs(taxes) + abs(profits) + abs(dividends)
        + abs(new_equity) + abs(liq_equity) + abs(bad_debt) + abs(depo_intrst) + abs(loan_intrst) + abs(depo_chg)
        + abs(loan_chg) + abs(gov_debt_chg) + abs(gov_depo_chg);

    // capital transaction sub-matrix column sums (net lending)
    let workers_nl = -cc + g + w - tax_w - tax_div + div_1 - c_entry_1 + c_exit_1 + r * sav_acc_1;
    let firms1nl = net_pi1 - div1_1 + c_entry1_1 - c_exit1_1 + bad_deb1_1;
    let firms2nl = -inom + net_pi2 - div2_1 + c_entry2_1 - c_exit2_1 + bad_deb2_1;
    let bank_nl = -pi_b - bad_deb_1 - i_db + i_b;
    let govt_nl = -g + tax + pi_b;

    v[4] = workers_nl + firms1nl + firms2nl + bank_nl + govt_nl;

    // transaction-flow matrix column sums (net flows/changes in stocks)
    let workers = workers_nl - d_sav_acc;
    let firms1c = s1 - w1 - tax1 - net_pi1 + id1 - i1;
    let firms1k = firms1nl - d_nw1 + d_deb1;
    let firms2c = s2 - w2 - tax2 - net_pi2 + id2 - i2;
    let firms2k = firms2nl - d_nw2 + d_deb2;
    let bank = bank_nl - d_mb - d_deb + d_depo_g;
    let govt = govt_nl + d_deb - d_depo_g;

    v[5] = abs(workers) + abs(firms1c) + abs(firms1k) + abs(firms2c) + abs(firms2k) + abs(bank) + abs(govt);

    v[6] = c.vs(par, "GDPnom"); // reference for % of GDP results

    v[1] = round_tol(v[1] / v[6], 0.0, SFCTHRD); // round close to zero values
    v[2] = round_tol(v[2] / v[6], 0.0, SFCTHRD);
    v[3] = round_tol(v[3] / v[6], 0.0, SFCTHRD);
    v[4] = round_tol(v[4] / v[6], 0.0, SFCTHRD);
    v[5] = round_tol(v[5] / v[6], 0.0, SFCTHRD);

    // (the C++ only logs the checks when called from the Stats object)

    v[5] / 2.0
}


// ---------------------------------------------------------------------------------------------- shared helpers

/// The common start of the 'testXXX' equations: read the time range, become a parameter after the last period, and
/// give up (result 0) outside the range. Returns v[1], v[2] (initial and final period) when the test is to be run.
fn test_range(c: &mut Ctx, ini: &str, end: &str) -> Option<(f64, f64)> {
    let v1 = c.v(ini);
    let v2 = c.v(end);

    if c.t() >= v2 {
        c.parameter(); // compute for the last time
    }

    if c.t() < v1 || v2 == 0.0 {
        return None; // END_EQUATION( 0 )
    }
    Some((v1, v2))
}

/// The three loops over the nonNeg / posit / finite arrays (`all` is the three concatenated).
fn check_arrays(non_neg: &[f64], posit: &[f64], finite: &[f64], errors: &mut i32) {
    for &x in non_neg {
        check_error(x < 0.0, errors);
    }
    for &x in posit {
        check_error(x <= 0.0, errors);
    }
    for &x in non_neg.iter().chain(posit).chain(finite) {
        check_error(!x.is_finite(), errors);
    }
}

/// C's "%g".
fn fmt_g(x: f64) -> String {
    if x.is_nan() {
        return if x.is_sign_negative() { "-nan".into() } else { "nan".into() };
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf".into() } else { "inf".into() };
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let e = format!("{:.5e}", x);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    if exp < -4 || exp >= 6 {
        let m = if mant.contains('.') { mant.trim_end_matches('0').trim_end_matches('.') } else { mant };
        format!("{}e{}{:02}", m, if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        let s = format!("{:.*}", (5 - exp).max(0) as usize, x);
        if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
    }
}

thread_local! {
    static COUNTRY: Cell<(f64, f64, f64, f64)> = Cell::new((0.0, 0.0, 0.0, 0.0)); // iniGDP, iniA, iniDeb, iniSavAcc
    static COUNTRY_ERRORS: Cell<i32> = Cell::new(0);
    static FIN_ERRORS: Cell<i32> = Cell::new(0);
    static LAB_ERRORS: Cell<i32> = Cell::new(0);
    static SEC1_ERRORS: Cell<i32> = Cell::new(0);
    static SEC2_ERRORS: Cell<i32> = Cell::new(0);
    static FIRMS1_FILE: RefCell<Option<File>> = RefCell::new(None);
    static FIRMS1_STATE: Cell<(f64, f64, i32)> = Cell::new((0.0, 0.0, 0)); // iniAtau, iniBtau, errorsTot
    static FIRMS1_ENTR: RefCell<BTreeMap<i32, ObjId>> = RefCell::new(BTreeMap::new());
    static FIRMS2_FILE: RefCell<Option<File>> = RefCell::new(None);
    static FIRMS2_STATE: Cell<(f64, f64, f64, i32)> = Cell::new((0.0, 0.0, 0.0, 0)); // iniK, iniNW2, iniDeb2, errorsTot
    static FIRMS2_ENTR: RefCell<BTreeMap<i32, ObjId>> = RefCell::new(BTreeMap::new());
}

fn write_file(f: &'static std::thread::LocalKey<RefCell<Option<File>>>, s: &str) {
    f.with(|f| {
        if let Some(f) = f.borrow_mut().as_mut() {
            f.write_all(s.as_bytes()).unwrap();
        }
    });
}

// ---------------------------------------------------------------------------------------------- country-level tests

/// Print detailed statistics of country macro (!=0 if error is found). Set the time range in 'testCtIni' and 'testCtEnd'.
fn test_country(c: &mut Ctx) -> f64 {
    let Some((v1, v2)) = test_range(c, "testCtIni", "testCtEnd") else { return 0.0 };
    let v3 = 1.0 + v2 - v1; // number of periods
    let t = c.t();

    let mut errors = 0; // error counter
    let par = c.parent();
    let (cap, con, mac, sec) = (capsecl1(c), consecl1(c), macstal1(c), secstal1(c));

    let a = c.vs(par, "A");
    let cc = c.vs(par, "C");
    let creal = c.vs(par, "Creal");
    let cd = c.vs(par, "Cd");
    let deb = c.vs(par, "Deb");
    let def = c.vs(par, "Def");
    let div = c.vs(par, "Div");
    let eq = c.vs(par, "Eq");
    let g = c.vs(par, "G");
    let gdp_real = c.vs(par, "GDPreal");
    let gdp_nom = c.vs(par, "GDPnom");
    let sav = c.vs(par, "Sav");
    let sav_acc = c.vs(par, "SavAcc");
    let tax = c.vs(par, "Tax");
    let tax_div = c.vs(par, "TaxDiv");
    let c_entry = c.vs(par, "cEntry");
    let c_exit = c.vs(par, "cExit");
    let d_gdp = c.vs(par, "dGDP");
    let d_ab = c.vs(par, "dAb");
    let entry_exit = c.vs(par, "entryExit");
    let _ = (def, entry_exit);

    let l1 = c.vs(cap, "L1");
    let ppi = c.vs(cap, "PPI");
    let q1e = c.vs(cap, "Q1e");
    let pk0 = c.vs(cap, "pK0");

    let cpi = c.vs(con, "CPI");
    let d2d = c.vs(con, "D2d");
    let ireal = c.vs(con, "Ireal");
    let inom = c.vs(con, "Inom");
    let l2 = c.vs(con, "L2");
    let n = c.vs(con, "N");
    let q2e = c.vs(con, "Q2e");
    let d_nnom = c.vs(con, "dNnom");
    let pc0 = c.vs(con, "pC0");

    let gdi = c.vs(mac, "GDI");
    let d_a = c.vs(mac, "dA");

    let d_n = c.vs(sec, "dN");

    let non_neg = [div, eq, g, ireal, inom, n, sav, sav_acc, tax, tax_div, c_entry, c_exit];
    let posit = [a, cc, creal, cd, d2d, gdp_real, gdp_nom, gdi];
    let finite = [d_a, d_ab, d_gdp, d_n, d_nnom];

    // first period actions
    if t == v1 {
        COUNTRY.with(|s| s.set((gdp_real, a, deb, sav_acc)));
    }
    let (ini_gdp, ini_a, ini_deb, ini_sav_acc) = COUNTRY.with(|s| s.get());

    // national accounting
    check_arrays(&non_neg, &posit, &finite, &mut errors);

    check_error(d_a < -2.0 * TOL, &mut errors);

    check_error(
        a < min(q1e * pk0 / l1, q2e * pc0 / l2) || a > max(q1e * pk0 / l1, q2e * pc0 / l2),
        &mut errors,
    );

    check_error(cd.ceil() < cc.floor(), &mut errors);

    check_error(
        gdp_nom > (1.0 + 2.0 * TOL) * (q1e * ppi + q2e * cpi)
            || gdp_nom < (1.0 - 2.0 * TOL) * (q1e * ppi + q2e * cpi)
            || gdp_nom > (1.0 + TOL) * (cc + inom + d_nnom)
            || gdp_nom < (1.0 - TOL) * (cc + inom + d_nnom),
        &mut errors,
    );

    check_error(gdp_nom > (1.0 + 2.0 * TOL) * gdi || gdp_nom < (1.0 - 2.0 * TOL) * gdi, &mut errors);

    check_error(sav / gdp_nom > 5.0 * TOL, &mut errors);

    // forced savings, public debt, firms equity and dynamics
    check_error(c_entry / gdp_nom > 2.0 * TOL, &mut errors);

    check_error(deb / gdp_nom > 100.0 * TOL, &mut errors);

    // SFC check
    c.recalc("testSFC");
    if c.v("testSFC") > 0.0 {
        errors += 1;
    }

    let mut errors_tot = COUNTRY_ERRORS.with(|s| s.get()) + errors;

    // last period actions
    if t == v2 {
        let w4 = ((gdp_real + 1.0).ln() - (ini_gdp + 1.0).ln()) / v3;
        let w5 = (a.ln() - ini_a.ln()) / v3;
        let w6 = ((deb + 1.0).ln() - (ini_deb + 1.0).ln()) / v3;
        let w7 = ((sav_acc + 1.0).ln() - (ini_sav_acc + 1.0).ln()) / v3;

        check_error(w4 < TOL / 20.0 || w5 < TOL / 20.0, &mut errors_tot);

        check_error(w6 > (1.0 + TOL) * w4, &mut errors_tot);

        check_error(w7 > (1.0 + TOL) * w4, &mut errors_tot);
    }
    COUNTRY_ERRORS.with(|s| s.set(errors_tot));

    errors as f64
}

// ---------------------------------------------------------------------------------------------- sector-level tests

/// Print detailed statistics of financial sector (!=0 if error is found). Set the time range in 'testFtIni' and 'testFtEnd'.
fn test_fin(c: &mut Ctx) -> f64 {
    let Some((_v1, _v2)) = test_range(c, "testFtIni", "testFtEnd") else { return 0.0 };

    let mut errors = 0; // error counter
    let par = c.parent();
    let (cap, con, fin, mac, sec) = (capsecl1(c), consecl1(c), finsecl1(c), macstal1(c), secstal1(c));

    let bad_deb = c.vs(fin, "BadDeb");
    let bad_deb1 = c.vs(fin, "BadDeb1");
    let bad_deb2 = c.vs(fin, "BadDeb2");
    let depo = c.vs(fin, "Depo");
    let depo_g = c.vs(fin, "DepoG");
    let loans = c.vs(fin, "Loans");
    let pi_b = c.vs(fin, "PiB");
    let i_b = c.vs(fin, "iB");
    let i_db = c.vs(fin, "iDb");
    let _r = c.vs(fin, "r");

    let bad_deb_1 = c.vls(fin, "BadDeb", 1);
    let depo_1 = c.vls(fin, "Depo", 1);
    let depo_g_1 = c.vls(fin, "DepoG", 1);
    let loans_1 = c.vls(fin, "Loans", 1);

    let cd = c.vs(mac, "CD");
    let cdc = c.vs(mac, "CDc");
    let cs = c.vs(mac, "CS");
    let bda = c.vs(sec, "Bda");

    let deb = c.vs(par, "Deb");
    let deb_1 = c.vls(par, "Deb", 1);
    let sav_acc = c.vs(par, "SavAcc");

    let deb1 = c.vs(cap, "Deb1");
    let nw1 = c.vs(cap, "NW1");

    let deb2 = c.vs(con, "Deb2");
    let nw2 = c.vs(con, "NW2");

    let non_neg = [cd, cdc, cs, deb1, deb2, depo, depo_g, loans, bad_deb, bad_deb1, bad_deb2, bda, sav_acc, nw1, nw2, i_b, i_db];
    let posit: [f64; 0] = [];
    let finite = [nw1, nw2];

    // interest rate structure and bank assets and liabilities
    check_arrays(&non_neg, &posit, &finite, &mut errors);

    // try to account for deposits from loans of entrant firms (very crude)
    check_error(abs(nw1 + nw2 + sav_acc - depo - deb1 - deb2 + loans) / depo > TOL, &mut errors);

    check_error(cs > cd || cdc > cd, &mut errors);

    // bank cash-flow
    check_error(
        abs(-pi_b - bad_deb_1 - i_db + i_b - ((loans - depo) - (loans_1 - depo_1)) - (deb - deb_1) + (depo_g - depo_g_1)) > TOL,
        &mut errors,
    );

    FIN_ERRORS.with(|s| s.set(s.get() + errors));

    errors as f64
}

/// Print detailed statistics of the labor supply (!=0 if error is found). Set the time range in 'testLtIni' and 'testLtEnd'.
fn test_labor(c: &mut Ctx) -> f64 {
    let Some((_v1, _v2)) = test_range(c, "testLtIni", "testLtEnd") else { return 0.0 };

    let mut errors = 0; // error counter
    let par = c.parent();
    let (cap, con, fin, lab, labs) = (capsecl1(c), consecl1(c), finsecl1(c), labsupl1(c), labstal1(c));

    let l = c.vs(lab, "L");
    let ls = c.vs(lab, "Ls");
    let tax_w = c.vs(lab, "TaxW");
    let u = c.vs(lab, "U");
    let w_ = c.vs(lab, "W");
    let d_ub = c.vs(lab, "dUb");
    let w = c.vs(lab, "w");

    let vac = c.vs(labs, "V");
    let cc = c.vs(par, "C");
    let g = c.vs(par, "G");
    let _eq = c.vs(par, "Eq");
    let sav_acc = c.vs(par, "SavAcc");

    let div_1 = c.vls(par, "Div", 1);
    let sav_acc_1 = c.vls(par, "SavAcc", 1);
    let c_entry_1 = c.vls(par, "cEntry", 1);
    let c_exit_1 = c.vls(par, "cExit", 1);
    let r = c.vs(fin, "r");

    let l1 = c.vs(cap, "L1");
    let w1 = c.vs(cap, "W1");
    let l2 = c.vs(con, "L2");
    let w2 = c.vs(con, "W2");

    let non_neg = [tax_w, u, vac, w_, l1, w1, l2, w2];
    let posit = [l, ls, w];
    let finite = [d_ub];

    // summary
    check_arrays(&non_neg, &posit, &finite, &mut errors);

    check_error(u > 1.0 || vac > 1.0, &mut errors);

    check_error(l1 + l2 != l || ls < l, &mut errors);

    check_error(w1 + w2 < (1.0 - TOL / 10.0) * l * w || w1 + w2 > (1.0 + TOL / 10.0) * l * w, &mut errors);

    // finance
    check_error(
        abs(-cc + g + w_ - tax_w + div_1 - c_entry_1 + c_exit_1 + r * sav_acc_1 - (sav_acc - sav_acc_1)) > TOL,
        &mut errors,
    );

    LAB_ERRORS.with(|s| s.set(s.get() + errors));

    errors as f64
}

/// Print detailed statistics of capital-good sector (!=0 if error is found). Set the time range in 'test1StIni' and 'test1StEnd'.
fn test1sec(c: &mut Ctx) -> f64 {
    let Some((_v1, _v2)) = test_range(c, "test1StIni", "test1StEnd") else { return 0.0 };
    let mut v = [0.0f64; 7];
    let t = c.t();

    // scan firms for severe problems
    let (cap, lab, fin, sec) = (capsecl1(c), labsupl1(c), finsecl1(c), secstal1(c));
    let f1 = c.vs(cap, "F1");

    // vectors to save error firms (only their sizes are used)
    let (mut a_err, mut rd_err, mut c1_err, mut cli_err, mut sfc_c_err, mut sfc_k_err) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut errors = 0; // error counter
    v[4] = 0.0;
    v[5] = 0.0;
    v[6] = 0.0; // accumulators
    let mut cur = c.firsts(cap, "Firm1");
    while let Some(o) = cur {
        v[4] += c.vs(o, "_f1");
        v[5] += c.vs(o, "_L1");

        if c.vs(o, "_t1ent") < t {
            // ignore just entered firm
            v[6] += c.vs(o, "_L1rd");
        }

        if c.vs(o, "_Atau") <= TOL || c.vs(o, "_Btau") <= TOL / 10.0 {
            a_err += 1;
        }

        if c.vs(o, "_RD") < 0.0 {
            rd_err += 1;
        }

        if c.vs(o, "_c1") <= 0.0 || c.vs(o, "_p1") <= 0.0 {
            c1_err += 1;
        }

        if c.vs(o, "_t1ent") > t && c.vs(o, "_HC") + c.vs(o, "_NC") <= 0.0 {
            cli_err += 1;
        }

        let x = c.vs(o, "_S1") - c.vs(o, "_W1") - c.vs(o, "_Tax1") - (c.vs(o, "_Pi1") - c.vs(o, "_Tax1")) + c.vs(o, "_iD1") - c.vs(o, "_i1");
        if abs(x) > TOL {
            sfc_c_err += 1;
        }

        let x = (c.vs(o, "_Pi1") - c.vs(o, "_Tax1")) - c.vls(o, "_Div1", 1) - (c.vs(o, "_NW1") - c.vls(o, "_NW1", 1))
            + (c.vs(o, "_Deb1") - c.vls(o, "_Deb1", 1))
            + if c.vs(o, "_t1ent") == t { c.vs(o, "_Eq1") } else { 0.0 };
        if abs(x) > TOL {
            sfc_k_err += 1;
        }

        cur = c.next(o);
    }

    let a1 = c.vs(cap, "A1");
    let d1 = c.vs(cap, "D1");
    let deb1 = c.vs(cap, "Deb1");
    let div1 = c.vs(cap, "Div1");
    let eq1 = c.vs(cap, "Eq1");
    let jo1 = c.vs(cap, "JO1");
    let l1 = c.vs(cap, "L1");
    let l1d = c.vs(cap, "L1d");
    let l1d_rd = c.vs(cap, "L1dRD");
    let l1rd = c.vs(cap, "L1rd");
    let nw1 = c.vs(cap, "NW1");
    let ppi = c.vs(cap, "PPI");
    let pi1 = c.vs(cap, "Pi1");
    let q1 = c.vs(cap, "Q1");
    let q1e = c.vs(cap, "Q1e");
    let s1 = c.vs(cap, "S1");
    let tax1 = c.vs(cap, "Tax1");
    let w1 = c.vs(cap, "W1");
    let c_entry1 = c.vs(cap, "cEntry1");
    let c_exit1 = c.vs(cap, "cExit1");
    let entry1exit = c.vs(cap, "entry1exit");
    let i1 = c.vs(cap, "i1");
    let id1 = c.vs(cap, "iD1");
    let imi = c.vs(cap, "imi");
    let inn = c.vs(cap, "inn");

    let deb1_1 = c.vls(cap, "Deb1", 1);
    let div1_1 = c.vls(cap, "Div1", 1);
    let nw1_1 = c.vls(cap, "NW1", 1);
    let c_entry1_1 = c.vls(cap, "cEntry1", 1);
    let c_exit1_1 = c.vls(cap, "cExit1", 1);

    let bad_deb1_1 = c.vls(fin, "BadDeb1", 1);
    let cd1 = c.vs(sec, "CD1");
    let cd1c = c.vs(sec, "CD1c");
    let cs1 = c.vs(sec, "CS1");
    let ls = c.vs(lab, "Ls");
    let hh1 = c.vs(sec, "HH1");
    let hp1 = c.vs(sec, "HP1");
    let rd = c.vs(sec, "RD");
    let age1avg = c.vs(sec, "age1avg");

    let non_neg = [
        cd1, cd1c, cs1, d1, deb1, div1, eq1, jo1, l1, l1d, l1d_rd, l1rd, q1, q1e, s1, tax1, w1, c_entry1, c_exit1, i1, id1, imi, inn,
        hh1, hp1, rd, age1avg,
    ];
    let posit = [a1, ppi];
    let finite = [nw1, entry1exit, pi1];

    // innovation, productivity
    check_arrays(&non_neg, &posit, &finite, &mut errors);

    check_error(a_err as f64 > TOL * f1, &mut errors);

    check_error(rd_err as f64 > TOL * f1, &mut errors);

    check_error(q1e.floor() > q1 || q1.floor() > d1, &mut errors);

    // labor
    check_error(
        l1d_rd.ceil() < l1rd || l1rd.floor() > l1 || l1d.ceil() < jo1 || l1.floor() > ls || l1.ceil() < v[5] || l1rd.ceil() < v[6],
        &mut errors,
    );

    // cash flow
    check_error(s1 + rd < (1.0 - TOL) * w1, &mut errors);

    check_error(c1_err > 0, &mut errors);

    check_error(abs(s1 - w1 - tax1 - (pi1 - tax1) + id1 - i1) > TOL, &mut errors);

    check_error(sfc_c_err > 0, &mut errors);

    // finance
    check_error(cs1 > cd1 || cd1c > cd1, &mut errors);

    check_error(
        abs((pi1 - tax1) - div1_1 + c_entry1_1 - c_exit1_1 + bad_deb1_1 - (nw1 - nw1_1) + (deb1 - deb1_1)) > TOL,
        &mut errors,
    );

    check_error(sfc_k_err > 0, &mut errors);

    // competition
    check_error(v[4] < 1.0 - TOL / 10.0 || v[4] > 1.0 + TOL / 10.0, &mut errors);

    check_error(cli_err > 0, &mut errors);

    check_error(hh1 > 1.0 || hp1 > 2.0, &mut errors);

    SEC1_ERRORS.with(|s| s.set(s.get() + errors));

    errors as f64
}

/// Print detailed statistics of consumption-good sector (!=0 if error is found). Set the time range in 'test2StIni' and 'test2StEnd'.
fn test2sec(c: &mut Ctx) -> f64 {
    let Some((_v1, _v2)) = test_range(c, "test2StIni", "test2StEnd") else { return 0.0 };
    let mut v = [0.0f64; 6];
    let t = c.t();

    // scan firms for severe problems
    let (con, lab, fin, sec) = (consecl1(c), labsupl1(c), finsecl1(c), secstal1(c));
    let f2 = c.vs(con, "F2"); // number of firms
    let mu20 = c.vs(con, "mu20"); // initial mark-up

    // vectors to save error firms (only their sizes are used)
    let (mut d2_err, mut k_err, mut l2_err, mut q2_err, w2_err, mut c2_err, mut mu2_err, mut sfc_c_err, mut sfc_k_err) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);

    let mut errors = 0; // error counter
    v[4] = 0.0;
    v[5] = 0.0; // accumulators
    let mut cur = c.firsts(con, "Firm2");
    while let Some(o) = cur {
        v[4] += c.vs(o, "_f2");
        v[5] += if c.vs(o, "_life2cycle") > 0.0 && c.vs(o, "_L2") <= 0.0 { 1.0 } else { 0.0 };

        if c.vs(o, "_life2cycle") > 0.0 && c.vls(o, "_K", 1) <= 0.0 {
            k_err += 1;
        }

        if c.vs(o, "_life2cycle") > 0.0 && c.vs(o, "_Q2d") + c.vls(o, "_N", 1) <= 0.0 {
            d2_err += 1;
        }

        if c.vs(o, "_Q2d") > 0.0 && c.vs(o, "_L2") > 0.0 && c.vs(o, "_Q2e") <= 0.0 {
            q2_err += 1;
        }

        if c.vs(o, "_Q2e") > 0.0 && c.vs(o, "_L2") <= 0.0 {
            l2_err += 1;
        }

        if c.vs(o, "_c2") <= 0.0 || c.vs(o, "_p2") <= 0.0 {
            c2_err += 1;
        }

        if c.vs(o, "_mu2") < mu20 / 5.0 || c.vs(o, "_mu2") > mu20 * 5.0 {
            mu2_err += 1;
        }

        let x = c.vs(o, "_S2") - c.vs(o, "_W2") - c.vs(o, "_Tax2") - (c.vs(o, "_Pi2") - c.vs(o, "_Tax2")) + c.vs(o, "_iD2") - c.vs(o, "_i2");
        if abs(x) > TOL {
            sfc_c_err += 1;
        }

        let x = -c.vs(o, "_Inom") + (c.vs(o, "_Pi2") - c.vs(o, "_Tax2")) - c.vls(o, "_Div2", 1) - (c.vs(o, "_NW2") - c.vls(o, "_NW2", 1))
            + (c.vs(o, "_Deb2") - c.vls(o, "_Deb2", 1))
            + if c.vs(o, "_t2ent") == t { c.vs(o, "_Eq2") } else { 0.0 };
        if abs(x) > TOL {
            sfc_k_err += 1;
        }

        cur = c.next(o);
    }

    let a2 = c.vs(con, "A2");
    let ci = c.vs(con, "CI");
    let cpi = c.vs(con, "CPI");
    let deb2 = c.vs(con, "Deb2");
    let div2 = c.vs(con, "Div2");
    let d2 = c.vs(con, "D2");
    let d2d = c.vs(con, "D2d");
    let d2e = c.vs(con, "D2e");
    let eavg = c.vs(con, "Eavg");
    let ei = c.vs(con, "EI");
    let eq2 = c.vs(con, "Eq2");
    let id = c.vs(con, "Id");
    let inom = c.vs(con, "Inom");
    let ireal = c.vs(con, "Ireal");
    let jo2 = c.vs(con, "JO2");
    let k = c.vs(con, "K");
    let kd = c.vs(con, "Kd");
    let knom = c.vs(con, "Knom");
    let l2 = c.vs(con, "L2");
    let l2d = c.vs(con, "L2d");
    let n = c.vs(con, "N");
    let nw2 = c.vs(con, "NW2");
    let pi2 = c.vs(con, "Pi2");
    let q2 = c.vs(con, "Q2");
    let q2d = c.vs(con, "Q2d");
    let q2e = c.vs(con, "Q2e");
    let q2p = c.vs(con, "Q2p");
    let q2u = c.vs(con, "Q2u");
    let s2 = c.vs(con, "S2");
    let si = c.vs(con, "SI");
    let tax2 = c.vs(con, "Tax2");
    let w2 = c.vs(con, "W2");
    let c2 = c.vs(con, "c2");
    let c2e = c.vs(con, "c2e");
    let d_cpi = c.vs(con, "dCPI");
    let c_entry2 = c.vs(con, "cEntry2");
    let c_exit2 = c.vs(con, "cExit2");
    let entry2exit = c.vs(con, "entry2exit");
    let i2 = c.vs(con, "i2");
    let id2 = c.vs(con, "iD2");
    let l2avg = c.vs(con, "l2avg");
    let old_vint = c.vs(con, "oldVint");
    let p2avg = c.vs(con, "p2avg");

    let deb2_1 = c.vls(con, "Deb2", 1);
    let div2_1 = c.vls(con, "Div2", 1);
    let kavb = c.vls(con, "K", 1);
    let nw2_1 = c.vls(con, "NW2", 1);
    let c_entry2_1 = c.vls(con, "cEntry2", 1);
    let c_exit2_1 = c.vls(con, "cExit2", 1);

    let bad_deb2_1 = c.vls(fin, "BadDeb2", 1);
    let cd2 = c.vs(sec, "CD2");
    let cd2c = c.vs(sec, "CD2c");
    let cs2 = c.vs(sec, "CS2");
    let ls = c.vs(lab, "Ls");
    let hh2 = c.vs(sec, "HH2");
    let hp2 = c.vs(sec, "HP2");
    let age2avg = c.vs(sec, "age2avg");
    let mu2avg = c.vs(sec, "mu2avg");

    let non_neg = [
        cd2, cd2c, cs2, cpi, ci, d2, d2d, d2e, deb2, div2, ei, eq2, id, inom, ireal, jo2, k, kavb, kd, knom, l2, l2d, n, q2, q2d, q2e,
        q2p, q2u, si, s2, tax2, w2, c_entry2, c_exit2, entry2exit, i2, id2, l2avg, hh2, hp2,
    ];
    let posit = [a2, -eavg, old_vint, p2avg, age2avg, mu2avg];
    let finite = [nw2, d_cpi, pi2];

    // capital and investment
    check_arrays(&non_neg, &posit, &finite, &mut errors);

    check_error(k_err > 0, &mut errors);

    check_error(si + ei > id, &mut errors);

    check_error(si > kavb || k > kavb + ei + si, &mut errors);

    // productivity
    check_error(c2e < (1.0 - 3.0 * TOL * q2 / q2e) * c2, &mut errors);

    // production
    check_error(q2e > q2 || q2 > q2d || q2e > k, &mut errors);

    check_error(c2_err > 0, &mut errors);

    check_error(q2_err > 0, &mut errors);

    // labor
    check_error(l2d.ceil() < jo2 || l2.floor() > ls, &mut errors);

    check_error(l2_err > 0, &mut errors);

    // cash flow
    check_error(w2_err > 0, &mut errors);

    check_error(s2 < (1.0 - TOL) * w2, &mut errors);

    check_error(v[5] / f2 > TOL, &mut errors);

    check_error(abs(s2 - w2 - tax2 - (pi2 - tax2) + id2 - i2) > TOL, &mut errors);

    check_error(sfc_c_err > 0, &mut errors);

    // finance
    check_error(cs2 > cd2 || cd2c > cd2, &mut errors);

    check_error(
        abs(-inom + (pi2 - tax2) - div2_1 + c_entry2_1 - c_exit2_1 + bad_deb2_1 - (nw2 - nw2_1) + (deb2 - deb2_1)) > TOL,
        &mut errors,
    );

    check_error(sfc_k_err > 0, &mut errors);

    // market
    check_error(d2 > (1.0 + TOL) * d2d, &mut errors);

    check_error(d2_err > 0, &mut errors);

    check_error(mu2_err > 0, &mut errors);

    // competition
    check_error(v[4] < 1.0 - TOL / 10.0 || v[4] > 1.0 + TOL / 10.0, &mut errors);

    check_error(hh2 > 1.0 || hp2 > 2.0, &mut errors);

    SEC2_ERRORS.with(|s| s.set(s.get() + errors));

    errors as f64
}

// ---------------------------------------------------------------------------------------------- firm-level tests

/// Print detailed statistics of firms in capital-good sector (!=0 if error).
/// Set the firm range by firm _ID1 in 'test1idIni' and 'test1idEnd'. Set the time range in 'test1tIni' and 'test1tEnd'.
/// If 'test1idIni' is zero, list up to 'test1idEnd' firms entered from 'test1tIni'.
fn test1firm(c: &mut Ctx) -> f64 {
    let Some((v1, v2)) = test_range(c, "test1tIni", "test1tEnd") else { return 0.0 };
    let t = c.t();

    let v3 = c.v("test1idIni");
    let v4 = c.v("test1idEnd");

    let v5 = 1.0 + v2 - v1; // number of periods
    let k: i32 = if v3 > 0.0 { 1.0 + v4 - v3 } else { v4 } as i32; // number of firms

    if t == v1 {
        // (re)create the file; don't reopen if already open
        if FIRMS1_FILE.with(|f| f.borrow().is_none()) {
            let mut f = File::create(TEST1FILE).expect("firms1.csv");
            write!(f, "{},{},{},{}
", "t,ID1,t1ent,Client", "RD,Atau,Btau,c1,D1,Q1,Q1e,L1d,L1", "Pi1,NW1,Deb1,Deb1max,CS1,CD1c", "HC,NC,BC,p1,S1,f1").unwrap();
            FIRMS1_FILE.with(|g| *g.borrow_mut() = Some(f));
        }
    }

    let mut errors = 0; // error counter
    let cap = capsecl1(c);
    let mut cur = c.firsts(cap, "Firm1");
    while let Some(o) = cur {
        let next = c.next(o);
        'body: {
            let id = c.vs(o, "_ID1") as i32;
            let h = c.vs(o, "_t1ent") as i32;

            // print ID range
            if v3 > 0.0 && ((id as f64) < v3 || (id as f64) > v4) {
                break 'body;
            } else if v3 <= 0.0 {
                // print entrants in certain period
                let skip = FIRMS1_ENTR.with(|e| {
                    let mut e = e.borrow_mut();
                    if !e.contains_key(&id) {
                        // not in list: too old or list is full?
                        if (h as f64) < v1 || e.len() >= (v4 as u32 as usize) {
                            true
                        } else {
                            e.insert(id, o); // add entrant in to show list
                            false
                        }
                    } else {
                        false
                    }
                });
                if skip {
                    break 'body;
                }
            }

            let j = c.counts(o, "Cli") as i32;

            let atau = c.vs(o, "_Atau");
            let btau = c.vs(o, "_Btau");
            let bc = c.vs(o, "_BC");
            let cd1 = c.vs(o, "_CD1");
            let cd1c = c.vs(o, "_CD1c");
            let cs1 = c.vs(o, "_CS1");
            let d1 = c.vs(o, "_D1");
            let deb1 = c.vs(o, "_Deb1");
            let deb1max = c.vs(o, "_Deb1max");
            let div1 = c.vs(o, "_Div1");
            let eq1 = c.vs(o, "_Eq1");
            let hc = c.vs(o, "_HC");
            let l1 = c.vs(o, "_L1");
            let l1d = c.vs(o, "_L1d");
            let l1d_rd = c.vs(o, "_L1dRD");
            let l1rd = c.vs(o, "_L1rd");
            let nc = c.vs(o, "_NC");
            let nw1 = c.vs(o, "_NW1");
            let pi1 = c.vs(o, "_Pi1");
            let q1 = c.vs(o, "_Q1");
            let q1e = c.vs(o, "_Q1e");
            let rd = c.vs(o, "_RD");
            let s1 = c.vs(o, "_S1");
            let tax1 = c.vs(o, "_Tax1");
            let w1 = c.vs(o, "_W1");
            let c1 = c.vs(o, "_c1");
            let f1 = c.vs(o, "_f1");
            let i1 = c.vs(o, "_i1");
            let id1 = c.vs(o, "_iD1");
            let imi = c.vs(o, "_imi");
            let inn = c.vs(o, "_inn");
            let p1 = c.vs(o, "_p1");

            let deb1_1 = c.vls(o, "_Deb1", 1);
            let div1_1 = c.vls(o, "_Div1", 1);
            let nw1_1 = c.vls(o, "_NW1", 1);

            let non_neg = [cs1, cd1, cd1c, div1, eq1, hc, nc, rd, d1, q1, q1e, bc, l1, l1d, l1d_rd, l1rd, deb1, deb1max, s1, tax1, w1, f1, i1, id1, imi, inn];
            let posit = [atau, btau, c1, p1];
            let finite = [nw1, pi1];

            // first period actions (single-firm analysis only)
            if k == 1 && t == v1 {
                FIRMS1_STATE.with(|s| { let (_, _, e) = s.get(); s.set((atau, btau, e)) });
            }

            let mut line = format!("{},{},{},{}", fmt_g(t), id, h, j);

            check_arrays(&non_neg, &posit, &finite, &mut errors);

            // innovation, productivity
            line += &format!(",{},{},{},{},{},{},{},{},{}", fmt_g(rd), fmt_g(atau), fmt_g(btau), fmt_g(c1), fmt_g(d1), fmt_g(q1), fmt_g(q1e), fmt_g(l1d), fmt_g(l1));

            check_error(rd <= 0.0, &mut errors);

            check_error(btau > atau || atau < TOL || btau < TOL / 10.0, &mut errors);

            // finance
            line += &format!(",{},{},{},{},{},{}", fmt_g(pi1), fmt_g(nw1), fmt_g(deb1), fmt_g(deb1max), fmt_g(cs1), fmt_g(cd1c));

            check_error(cs1 > cd1 || cd1c > cd1, &mut errors);

            check_error(abs(s1 - w1 - tax1 - (pi1 - tax1) + id1 - i1) > TOL, &mut errors);

            check_error(
                abs((pi1 - tax1) - div1_1 - (nw1 - nw1_1) + (deb1 - deb1_1) + if h as f64 == t { eq1 } else { 0.0 }) > TOL,
                &mut errors,
            );

            // market and client dynamics
            line += &format!(",{},{},{},{},{},{}", fmt_g(hc), fmt_g(nc), fmt_g(bc), fmt_g(p1), fmt_g(s1), fmt_g(f1));

            check_error(j as f64 > hc + nc, &mut errors);

            check_error((j as f64) < bc, &mut errors);

            // last period actions (single-firm analysis only): only logged
            let _ = v5;

            line += "\n";
            write_file(&FIRMS1_FILE, &line);
        }
        cur = next;
    }

    let errors_tot = FIRMS1_STATE.with(|s| s.get().2) + errors;
    FIRMS1_STATE.with(|s| { let (a, b, _) = s.get(); s.set((a, b, errors_tot)) });

    if t == v2 {
        FIRMS1_FILE.with(|f| *f.borrow_mut() = None); // fclose( firms1 )
    }

    errors as f64
}

/// Print detailed statistics of firms in consumption-good sector (!=0 if error).
/// Set the firm range by firm _ID2 in 'test2idIni' and 'test2idEnd'. Set the time range in 'test2tIni' and 'test2tEnd'.
/// If 'test2idIni' is zero, list up to 'test2idEnd' firms entered from 'test2tIni'.
fn test2firm(c: &mut Ctx) -> f64 {
    let Some((v1, v2)) = test_range(c, "test2tIni", "test2tEnd") else { return 0.0 };
    let t = c.t();

    let v3 = c.v("test2idIni");
    let v4 = c.v("test2idEnd");

    let _v5 = 1.0 + v2 - v1; // number of periods
    let k: i32 = if v3 > 0.0 { 1.0 + v4 - v3 } else { v4 } as i32; // number of firms

    if t == v1 {
        // (re)create the file; don't reopen if already open
        if FIRMS2_FILE.with(|f| f.borrow().is_none()) {
            let mut f = File::create(TEST2FILE).expect("firms2.csv");
            write!(
                f,
                "{},{},{},{},{}\n",
                "t,ID2,t2ent,life2cycle,Broch,Vint,pVint", "Kd,Kavb,EId,SId,EI,SI,CI,K", "D2e,Q2d,Q2,Q2e,L2d,L2,c2", "Pi2,NW2,Deb2,Deb2max,CS2,CD2c", "mu2,p2,D2,S2,W2,f2,N"
            )
            .unwrap();
            FIRMS2_FILE.with(|g| *g.borrow_mut() = Some(f));
        }
    }

    let con = consecl1(c);
    let eta = c.vs(con, "eta"); // machine technical life
    let iota = c.vs(con, "iota"); // production slack
    let m2 = c.vs(con, "m2"); // machine scale
    let mu20 = c.vs(con, "mu20"); // initial mark-up

    let mut errors = 0; // error counter
    let mut cur = c.firsts(con, "Firm2");
    while let Some(o) = cur {
        let next = c.next(o);
        'body: {
            let j = c.vs(o, "_ID2") as i32;
            let h = c.vs(o, "_t2ent") as i32;

            // print ID range
            if v3 > 0.0 && ((j as f64) < v3 || (j as f64) > v4) {
                break 'body;
            } else if v3 <= 0.0 {
                // print entrants in certain period
                let skip = FIRMS2_ENTR.with(|e| {
                    let mut e = e.borrow_mut();
                    if !e.contains_key(&j) {
                        // not in list: too old or list is full?
                        if (h as f64) < v1 || e.len() >= (v4 as u32 as usize) {
                            true
                        } else {
                            e.insert(j, o); // add entrant in to show list
                            false
                        }
                    } else {
                        false
                    }
                });
                if skip {
                    break 'body;
                }
            }

            // scan vintages for severe problems
            let l2_ = c.vs(o, "_L2");
            let q2e_ = c.vs(o, "_Q2e");

            // vectors to save error firms (only their sizes are used)
            let (mut id_err, mut t_vint_err, mut qvint_err, mut lvint_err) = (0usize, 0usize, 0usize, 0usize);

            let mut v7 = 0.0;
            let mut v8 = 0.0; // accumulators
            let mut cur1 = c.firsts(o, "Vint");
            while let Some(vi) = cur1 {
                v7 += c.vs(vi, "__nVint");
                v8 += c.vs(vi, "__Qvint");

                if c.vs(vi, "__IDvint") < h as f64 {
                    id_err += 1;
                }

                if (h == 1 && c.vs(vi, "__tVint") < 1.0 - eta) || (h > 1 && c.vs(vi, "__tVint") < h as f64) || c.vs(vi, "__tVint") > t {
                    t_vint_err += 1;
                }

                if c.vs(vi, "__Qvint") < 0.0 {
                    qvint_err += 1;
                }

                if c.vs(vi, "__toUseVint") == 0.0 && (c.vs(vi, "__LdVint") > 0.0 || c.vs(vi, "__Lvint") > 0.0) {
                    lvint_err += 1;
                }

                cur1 = c.next(vi);
            }
            let _ = v8;

            let n_broch = c.counts(o, "Broch");
            let n_vint = c.counts(o, "Vint");
            let top = c.hooks(o, TOPVINT);

            let a2 = c.vs(o, "_A2");
            let cd2 = c.vs(o, "_CD2");
            let cd2c = c.vs(o, "_CD2c");
            let ci = c.vs(o, "_CI");
            let cs2 = c.vs(o, "_CS2");
            let d2 = c.vs(o, "_D2");
            let d2d = c.vs(o, "_D2d");
            let d2e = c.vs(o, "_D2e");
            let deb2 = c.vs(o, "_Deb2");
            let deb2max = c.vs(o, "_Deb2max");
            let div2 = c.vs(o, "_Div2");
            let e = c.vs(o, "_E");
            let ei = c.vs(o, "_EI");
            let eid = c.vs(o, "_EId");
            let eq2 = c.vs(o, "_Eq2");
            let inom = c.vs(o, "_Inom");
            let jo2 = c.vs(o, "_JO2");
            let kk = c.vs(o, "_K");
            let kd = c.vs(o, "_Kd");
            let _knom = c.vs(o, "_Knom");
            let l2d = c.vs(o, "_L2d");
            let n = c.vs(o, "_N");
            let nw2 = c.vs(o, "_NW2");
            let pi2 = c.vs(o, "_Pi2");
            let q2 = c.vs(o, "_Q2");
            let q2d = c.vs(o, "_Q2d");
            let q2p = c.vs(o, "_Q2p");
            let q2pe = c.vs(o, "_Q2pe");
            let q2u = c.vs(o, "_Q2u");
            let rs2 = c.vs(o, "_RS2");
            let s2 = c.vs(o, "_S2");
            let si = c.vs(o, "_SI");
            let sid = c.vs(o, "_SId");
            let tax2 = c.vs(o, "_Tax2");
            let w2 = c.vs(o, "_W2");
            let c2 = c.vs(o, "_c2");
            let c2e = c.vs(o, "_c2e");
            let f2 = c.vs(o, "_f2");
            let i2 = c.vs(o, "_i2");
            let id2 = c.vs(o, "_iD2");
            let l2lost = c.vs(o, "_l2");
            let life2cycle = c.vs(o, "_life2cycle");
            let mu2 = c.vs(o, "_mu2");
            let p2 = c.vs(o, "_p2");
            let t2ent = c.vs(o, "_t2ent");

            let deb2_1 = c.vls(o, "_Deb2", 1);
            let div2_1 = c.vls(o, "_Div2", 1);
            let kavb = c.vls(o, "_K", 1);
            let n_1 = c.vls(o, "_N", 1);
            let nw2_1 = c.vls(o, "_NW2", 1);
            let p_vint = match top {
                Some(tv) if c.vs(tv, "__tVint") == t => c.vs(tv, "__pVint"),
                _ => 0.0,
            };

            let non_neg = [
                a2, cd2, cd2c, ci, cs2, d2, d2d, d2e, deb2, deb2max, div2, ei, eid, eq2, jo2, kk, kavb, kd, l2_, l2d, n, q2, q2d, q2e_, q2p, q2pe,
                q2u, rs2, s2, si, sid, tax2, w2, c2, c2e, f2, i2, id2, l2lost, life2cycle, p_vint, t2ent,
            ];
            let posit = [-e, mu2, p2];
            let finite = [nw2, pi2];

            // first period actions (single-firm analysis only)
            if k == 1 && t == v1 {
                FIRMS2_STATE.with(|s| { let (_, _, _, et) = s.get(); s.set((kk, nw2, deb2, et)) });
            }

            let mut line = format!("{},{},{},{},{},{},{}", fmt_g(t), j, h, fmt_g(life2cycle), fmt_g(n_broch), fmt_g(n_vint), fmt_g(p_vint));

            check_arrays(&non_neg, &posit, &finite, &mut errors);

            check_error(n_broch == 0.0, &mut errors);

            // capital and investment
            line += &format!(",{},{},{},{},{},{},{},{}", fmt_g(kd), fmt_g(kavb), fmt_g(eid), fmt_g(sid), fmt_g(ei), fmt_g(si), fmt_g(ci), fmt_g(kk));

            check_error(life2cycle > 0.0 && n_vint == 0.0, &mut errors);

            check_error(id_err > 0, &mut errors);

            check_error(t_vint_err > 0, &mut errors);

            check_error(sid > kavb || kk > kavb + ei + si || v7 * m2 > kk || (life2cycle > 0.0 && kk == 0.0), &mut errors);

            line += &format!(",{},{},{},{},{},{},{}", fmt_g(d2e), fmt_g(q2d), fmt_g(q2), fmt_g(q2e_), fmt_g(l2d), fmt_g(l2_), fmt_g(c2));

            check_error(
                (q2d > 0.0 && q2d.floor() > ((1.0 + iota) * d2e - n_1).ceil()) || q2 > q2d || q2e_ > kk || q2e_ > q2,
                &mut errors,
            );

            check_error(qvint_err > 0, &mut errors);

            check_error(l2d > 0.0 && l2_ == 0.0, &mut errors);

            check_error(lvint_err > 0, &mut errors);

            check_error(life2cycle > 0.0 && (c2 == 0.0 || c2e == 0.0), &mut errors);

            // finance
            line += &format!(",{},{},{},{},{},{}", fmt_g(pi2), fmt_g(nw2), fmt_g(deb2), fmt_g(deb2max), fmt_g(cs2), fmt_g(cd2c));

            check_error(cs2 > cd2 || cd2c > cd2, &mut errors);

            check_error(abs(s2 - w2 - tax2 - (pi2 - tax2) + id2 - i2) > TOL, &mut errors);

            check_error(
                abs(-inom + (pi2 - tax2) - div2_1 - (nw2 - nw2_1) + (deb2 - deb2_1) + if h as f64 == t { eq2 } else { 0.0 }) > TOL,
                &mut errors,
            );

            // market
            line += &format!(",{},{},{},{},{},{},{}", fmt_g(mu2), fmt_g(p2), fmt_g(d2), fmt_g(s2), fmt_g(w2), fmt_g(f2), fmt_g(n));

            check_error(mu2 > 5.0 * mu20, &mut errors);

            check_error(mu2 < mu20 / 5.0, &mut errors);

            // last period actions (single-firm analysis only): only logged

            line += "\n";
            write_file(&FIRMS2_FILE, &line);
        }
        cur = next;
    }

    let errors_tot = FIRMS2_STATE.with(|s| s.get().3) + errors;
    FIRMS2_STATE.with(|s| { let (a, b, d, _) = s.get(); s.set((a, b, d, errors_tot)) });

    if t == v2 {
        FIRMS2_FILE.with(|f| *f.borrow_mut() = None); // fclose( firms2 )
    }

    errors as f64
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("testSFC", EqDef::Fn(test_sfc)),
        ("testCountry", EqDef::Fn(test_country)),
        ("testFin", EqDef::Fn(test_fin)),
        ("testLabor", EqDef::Fn(test_labor)),
        ("test1sec", EqDef::Fn(test1sec)),
        ("test2sec", EqDef::Fn(test2sec)),
        ("test1firm", EqDef::Fn(test1firm)),
        ("test2firm", EqDef::Fn(test2firm)),
    ]);
}
