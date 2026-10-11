//! Example/SantAnna/K+S-original: the K+S macroeconomic ABM of Dosi et al. ("Keynes meets Schumpeter") as coded in
//! LSD by Marcelo Pereira (SFC version 0.1.2). The file split mirrors the C++ (`fun_KS.cpp` and `fun_KS_*.h`):
//!
//! | C++ file | Rust module |
//! |---|---|
//! | fun_KS.cpp (init, runCountry, timeStep, close_sim) | this file |
//! | fun_KS_class.h | `class` |
//! | fun_KS_support.h | `support` |
//! | fun_KS_country.h, _financial.h, _capital.h, _firm1.h, _consumption.h, _firm2.h, _vintage.h, _labor.h | `country`, `financial`, `capital`, `firm1`, `consumption`, `firm2`, `vintage`, `labor` |
//! | fun_KS_stats.h, fun_KS_test.h | `stats`, `test` |
//!
//! Notes on the port:
//!  * The C++ equation map is a `std::unordered_map` filled from an initializer list, which keeps the *first* of
//!    duplicate keys: `cEntry`, `cExit` (country) and `DepoG` (financial) have both an EQUATION and a later
//!    EQUATION_DUMMY; only the EQUATION is registered here.
//!  * EQUATION_DUMMY( X, "" ) is `dummy` (returns the variable's own value); with a non-empty second argument it is
//!    `EqDef::Dummy`.
//!  * No container in the model is ordered by pointer: `firm2map` is a `std::map<int, object *>` (by firm ID) and
//!    the other containers are vectors.
//!  * Verification: the configurations flag `_D1`, `_Q1e`, `_E`, `_Q2p`, `_Q2pe` and `_c2` for LSD's multi-threaded
//!    "parallel update" (`N` in their `Var:` line). `check.sh` runs `lsdNW` with all cores, so the ORDER of LSD's
//!    computation trace then depends on thread timing (its values and CSV do not: the multi-threaded CSV is identical
//!    to the single-threaded one and to this port's). To compare traces, make LSD single-threaded, either with
//!    `lsdNW ... -c 1` or with `parallel_disable` in the configuration (`MAX_STEP 600 0 0 0 0 0 0 1`).
//!  * C++ leaves the evaluation order of function arguments unspecified; GCC (x86-64) evaluates them right to left.
//!    Where `max`/`min` get several LSD value requests, the port requests them right to left (marked in the code).
//!    No trace tested so far depends on it (the requested variables are already computed at that point).
use crate::model::{Ctx, EqDef, Model};

mod capital;
mod class;
mod consumption;
mod country;
mod financial;
mod firm1;
mod firm2;
mod labor;
mod stats;
mod support;
mod test;
mod vintage;

use class::CountryE;
use support::new_vs;

/// EQUATION_DUMMY( X, "" ): the variable is written by other equations; its own computation keeps the value.
pub fn dummy(c: &mut Ctx) -> f64 {
    c.current()
}

/// Initialize the model, setting up K+S objects for each "Country" object. Also configures LSD main flags.
fn init(c: &mut Ctx) -> f64 {
    c.parameter(); // execute only once

    c.default_result(f64::NAN); // default equation result
    c.use_zero_instance(); // allow zero-instance objects
    c.no_search(); // don't perform variable search

    // (the FASTMODE logging switches only change the log display)

    let root = c.root();
    let mut cur = c.firsts(root, "Country"); // scan all country objects
    while let Some(o) = cur {
        c.vs(o, "initCountry"); // initialize country
        cur = c.next(o);
    }

    1.0
}

/// Execute the current time-step for all existing countries.
fn run_country(c: &mut Ctx) -> f64 {
    c.v("init"); // ensure initialization done

    let mut cur = c.first("Country");
    while let Some(o) = cur {
        c.vs(o, "timeStep");
        cur = c.next(o);
    }

    c.t()
}

/// Execute one time-step of one country in the K+S model.
/// Ensure the high-level scheduling of the model equation by forcing the computation of key variables in the
/// correct order.
fn time_step(c: &mut Ctx) -> f64 {
    let con = class::consecl0(c);
    let cap = class::capsecl0(c);
    let lab = class::labsupl0(c);
    let this = c.p;

    // consumption-good firms define expected demand, planned production,
    // labor demand, and desired investment for sector
    new_vs(c, con, "D2e"); // expected demand for goods
    new_vs(c, con, "Q2"); // planned goods production
    new_vs(c, con, "L2d"); // total desired labor
    new_vs(c, con, "Id"); // desired investment

    // capital-good firms do R&D, receive orders, define machine production,
    // and labor demand for sector
    new_vs(c, cap, "D1"); // orders for new machines
    new_vs(c, cap, "Q1"); // planned machine production
    new_vs(c, cap, "L1d"); // total desired labor

    // firms define open job positions and labor market define employed labor
    new_vs(c, cap, "JO1"); // open job positions in sector 1
    new_vs(c, con, "JO2"); // open job positions in sector 2
    new_vs(c, lab, "L"); // effective employed labor

    // production is adjusted to labor effectively hired by firms
    // and firms set prices
    new_vs(c, cap, "Q1e"); // effective machine production
    new_vs(c, con, "Q2e"); // effective goods production
    new_vs(c, cap, "p1avg"); // average machine prices
    new_vs(c, con, "p2avg"); // average goods prices

    // government decides expenditure and workers set all income to buying goods,
    // demand is matched to supply of goods, producing inventories or forced savings
    new_vs(c, this, "G"); // public total expenditures
    new_vs(c, con, "D2d"); // desired goods demand
    new_vs(c, con, "D2"); // fulfilled demand for goods
    new_vs(c, con, "N"); // accumulated inventories
    new_vs(c, this, "Sav"); // forced consumer savings

    // firms determine profits, taxes, cash flows, and update net wealths
    new_vs(c, cap, "Pi1"); // profits of sector 1
    new_vs(c, con, "Pi2"); // profits of sector 2
    new_vs(c, cap, "Tax1"); // tax paid by sector 1
    new_vs(c, con, "Tax2"); // tax paid by sector 2
    new_vs(c, cap, "NW1"); // net wealth of sector 1
    new_vs(c, con, "NW2"); // net wealth of sector 2

    // government collects taxes, compute public deficit or surplus form,
    // and adjust public debt accordingly
    new_vs(c, this, "Tax"); // total tax income
    new_vs(c, this, "Def"); // public total deficit
    new_vs(c, this, "Deb"); // public accumulated debt

    // macro variables are computed by aggregation of micro-level data
    new_vs(c, this, "GDPreal"); // real gross domestic product
    new_vs(c, this, "GDPnom"); // nominal gross domestic product

    // exit and entry happens in both sectors
    new_vs(c, this, "entryExit"); // net entry of firms

    c.t()
}

/// Garbage collection: C++ `close_sim`, reclaims the country extensions (Rust frees them with the model).
fn close_sim(m: &mut Model) {
    let root = m.root;
    let mut cur = m.cycle_first(root, "Country");
    while let Some(o) = cur {
        m.ext.remove(&o); // DELETE_EXTS( cur, countryE )
        cur = m.next(o);
    }
    let _ = std::any::type_name::<CountryE>();
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    let mut r: Vec<(&'static str, EqDef)> = vec![
        ("init", EqDef::Fn(init)),
        ("runCountry", EqDef::Fn(run_country)),
        ("timeStep", EqDef::Fn(time_step)),
        ("close_sim", EqDef::Hook(close_sim)),
    ];
    country::register(&mut r);
    financial::register(&mut r);
    capital::register(&mut r);
    firm1::register(&mut r);
    consumption::register(&mut r);
    firm2::register(&mut r);
    vintage::register(&mut r);
    labor::register(&mut r);
    stats::register(&mut r);
    test::register(&mut r);
    r
}
