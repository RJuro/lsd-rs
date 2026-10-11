//! fun_KS_capital.h: Capital-goods market object equations.
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Work force (labor) size employed by capital-good sector.
fn l1(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 7];
    let lab = labsupl1(c);
    let con = consecl1(c);

    v[1] = c.vs(lab, "Ls"); // available labor force
    v[2] = c.v("L1rd"); // R&D labor in sector 1
    v[3] = c.v("L1d"); // desired workers in sector 1
    v[4] = c.vs(con, "L2d"); // desired workers in sector 2
    v[5] = c.v("L1shortMax"); // max shortage allowed in sector

    v[2] = min(v[2], v[1]); // ignore demand over total labor
    v[3] = min(v[3], v[1]);
    v[4] = min(v[4], v[1]);

    if v[1] - v[2] < v[3] + v[4] {
        // labor shortage?
        v[6] = (v[1] - v[2]) / (v[3] + v[4]); // shortage factor

        if v[6] < 1.0 - v[5] {
            // over cap?
            v[6] = 1.0 - v[5]; // shortage on cap
        }
    } else {
        v[6] = 1.0; // no shortage
    }

    v[2] + (v[3] - v[2]) * v[6]
}

/// Rate of entry-exit of firms in capital-good sector.
/// Perform entry and exit of firms in the capital-good sector.
/// All relevant aggregate variables in sector must be computed before existing
/// firms are deleted, so all active firms in period are considered.
/// Also updates 'cEntry1', 'cExit1', 'exit1fail'.
fn entry1exit(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 8];
    let con = consecl1(c);

    c.vs(con, "K"); // ensure canceled orders acct'd
    c.update(); // ensure aggregates are computed

    let nw10u = c.v("NW10") * c.v("PPI") / c.v("pK0"); // minimum wealth in s. 1
    let n1 = c.v("n1"); // market participation period
    let f1 = c.v("F1") as i32; // number of firms

    let mut quit = vec![false; f1 as usize]; // vector of firms' quit status

    c.write("cEntry1", 0.0); // reset exit/entry accumulators
    c.write("cExit1", 0.0);

    // mark bankrupt and market-share-irrelevant firms to quit the market
    let mut h = f1; // initial number of firms
    v[1] = 0.0;
    v[3] = 0.0;
    let mut i: i32 = 0;
    let mut k: i32 = 0; // accum., counters, registers
    let mut cur = c.first("Firm1");
    while let Some(f) = cur {
        v[4] = c.vs(f, "_NW1"); // current net wealth

        if v[4] < 0.0 || c.t() >= c.vs(f, "_t1ent") + n1 {
            // bankrupt or incumbent?
            v[5] = 0.0;
            let mut j: i32 = 0;
            while (j as f64) < n1 {
                v[5] += c.vls(f, "_BC", j); // n1 periods customer number
                j += 1;
            }

            if v[4] < 0.0 || v[5] <= 0.0 {
                quit[i as usize] = true; // mark for likely exit
                h -= 1; // one less firm

                if v[5] > v[3] {
                    // best firm so far?
                    k = i; // save firm index
                    v[3] = v[5]; // and customer number
                }
            }
        }

        i += 1;
        cur = c.next(f);
    }

    // quit candidate firms exit, except the best one if all going to quit
    v[6] = 0.0;
    i = 0;
    let mut j: i32 = 0; // firm counters
    let mut cur = c.first("Firm1");
    while let Some(f) = cur {
        let next = c.next(f); // CYCLE_SAFE
        if quit[i as usize] {
            if h > 0 || i != k {
                // firm must exit?
                j += 1; // count exits
                if c.vs(f, "_NW1") < 0.0 {
                    // count bankruptcies
                    v[6] += 1.0;
                }

                exit_firm(c, f); // del obj & collect liq. value
            } else if h == 0 && i == k {
                // best firm must get new equity
                // new equity required
                v[7] = nw10u + c.vs(f, "_Deb1") - c.vs(f, "_NW1");
                v[1] += v[7];

                c.writes(f, "_Deb1", 0.0); // reset debt
                c.incrs(f, "_Eq1", v[7]); // add new equity
                c.incrs(f, "_NW1", v[7]);
            }
        }

        i += 1;
        cur = next;
    }

    c.v("f1rescale"); // redistribute exiting m.s.

    // replace exiting firms by entrants
    let p = c.p;
    entry_firm1(c, p, j, false); // add entrant-firm objects

    c.incr("cEntry1", v[1]); // add cost of additional equity
    let sec = secstal1(c);
    c.writes(sec, "exit1fail", v[6] / f1 as f64);

    c.v("f1rescale"); // redistribute entrant m.s.
    c.init_tsearcht("Firm1", i as f64); // prepare turbo search indexing

    j as f64 / f1 as f64
}

/// Labor productivity of capital-good sector.
fn a1(c: &mut Ctx) -> f64 {
    c.v("PPI"); // ensure m.s. are updated
    c.whtave("_Btau", "_f1")
}

/// Potential demand (orders) received by firms in capital-good sector.
fn d1(c: &mut Ctx) -> f64 {
    c.sum("_D1")
}

/// Total debt of capital-good sector.
fn deb1(c: &mut Ctx) -> f64 {
    c.sum("_Deb1")
}

/// Total dividends paid by firms in capital-good sector.
fn div1(c: &mut Ctx) -> f64 {
    c.v("Tax1"); // ensure dividends are computed
    c.sum("_Div1")
}

/// Equity hold by workers/households from firms in capital-good sector.
fn eq1(c: &mut Ctx) -> f64 {
    c.sum("_Eq1")
}

/// Open job positions in capital-good sector.
fn jo1(c: &mut Ctx) -> f64 {
    c.sum("_JO1")
}

/// Total labor demand from firms in capital-good sector. Includes R&D labor.
fn l1d(c: &mut Ctx) -> f64 {
    c.sum("_L1d")
}

/// R&D labor demand from firms in capital-good sector.
fn l1d_rd(c: &mut Ctx) -> f64 {
    c.sum("_L1dRD")
}

/// Total R&D labor employed by firms in capital-good sector. Apply hard limit to sectoral labor share if L1rdMax < 1.
fn l1rd(c: &mut Ctx) -> f64 {
    let lab = labsupl1(c);
    // min( V( "L1dRD" ), VS( .. ) * V( "L1rdMax" ) ): arguments evaluated right to left
    let b = c.vs(lab, "Ls") * c.v("L1rdMax");
    let a = c.v("L1dRD");
    min(a, b)
}

/// Total net wealth (free cash) of firms in capital-good sector.
fn nw1(c: &mut Ctx) -> f64 {
    c.sum("_NW1")
}

/// Total profits of capital-good sector.
fn pi1(c: &mut Ctx) -> f64 {
    c.sum("_Pi1")
}

/// Producer price index.
fn ppi(c: &mut Ctx) -> f64 {
    c.v("f1rescale"); // ensure m.s. computed/rescaled
    c.whtave("_p1", "_f1")
}

/// Total planned output of firms in capital-good sector.
fn q1(c: &mut Ctx) -> f64 {
    c.sum("_Q1")
}

/// Total effective real output (orders) of capital-good sector.
fn q1e(c: &mut Ctx) -> f64 {
    c.sum("_Q1e")
}

/// Total sales of capital-good sector.
fn s1(c: &mut Ctx) -> f64 {
    c.sum("_S1")
}

/// Total taxes paid by firms in capital-good sector.
fn tax1(c: &mut Ctx) -> f64 {
    c.sum("_Tax1")
}

/// Total wages paid by firms in capital-good sector.
fn w1(c: &mut Ctx) -> f64 {
    c.sum("_W1")
}

/// Interest paid by capital-good sector.
fn i1(c: &mut Ctx) -> f64 {
    c.sum("_i1")
}

/// Interest received from deposits by capital-good sector.
fn id1(c: &mut Ctx) -> f64 {
    c.sum("_iD1")
}

/// Imitation success rate in capital-good sector. Also ensures all innovation/imitation is done, brochures are
/// distributed and learning-by-doing skills are updated.
fn imi(c: &mut Ctx) -> f64 {
    c.sum("_Atau"); // ensure innovation is done
    c.sum("_NC"); // ensure brochures distributed
    let x = c.sum("_imi");
    x / c.v("F1")
}

/// Innovation success rate in capital-good sector. Also ensures all innovation/imitation is done, brochures are
/// distributed and learning-by-doing skills are updated.
fn inn(c: &mut Ctx) -> f64 {
    c.v("imi"); // ensure innovation is done
    let x = c.sum("_inn");
    x / c.v("F1")
}

/// Average price charged in capital-good sector.
fn p1avg(c: &mut Ctx) -> f64 {
    c.ave("_p1")
}

/// Rescale market shares in capital-good sector to ensure adding to 1.
/// To be called after market shares are changed in '_f1' and 'entry1exit'.
fn f1rescale(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 3];

    v[1] = c.sum("_f1"); // add-up market shares

    if round_tol(v[1], 1.0, 0.001) == 1.0 {
        // ignore rounding errors
        return v[1];
    }

    v[0] = 0.0; // accumulator

    if v[1] > 0.0 {
        // production ok?
        let mut cur = c.first("Firm1");
        while let Some(f) = cur {
            // rescale to add-up to 1
            v[2] = c.vs(f, "_f1") / v[1]; // rescaled market share
            v[0] += v[2];
            c.writes(f, "_f1", v[2]); // save updated m.s.
            cur = c.next(f);
        }
    } else {
        v[2] = 1.0 / c.count("Firm1"); // firm fair share

        let mut cur = c.first("Firm1");
        while let Some(f) = cur {
            // rescale to add-up to 1
            v[0] += v[2];
            c.writes(f, "_f1", v[2]);
            cur = c.next(f);
        }
    }

    v[0]
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("L1", EqDef::Fn(l1)),
        ("entry1exit", EqDef::Fn(entry1exit)),
        ("A1", EqDef::Fn(a1)),
        ("D1", EqDef::Fn(d1)),
        ("Deb1", EqDef::Fn(deb1)),
        ("Div1", EqDef::Fn(div1)),
        ("Eq1", EqDef::Fn(eq1)),
        ("JO1", EqDef::Fn(jo1)),
        ("L1d", EqDef::Fn(l1d)),
        ("L1dRD", EqDef::Fn(l1d_rd)),
        ("L1rd", EqDef::Fn(l1rd)),
        ("NW1", EqDef::Fn(nw1)),
        ("Pi1", EqDef::Fn(pi1)),
        ("PPI", EqDef::Fn(ppi)),
        ("Q1", EqDef::Fn(q1)),
        ("Q1e", EqDef::Fn(q1e)),
        ("S1", EqDef::Fn(s1)),
        ("Tax1", EqDef::Fn(tax1)),
        ("W1", EqDef::Fn(w1)),
        ("i1", EqDef::Fn(i1)),
        ("iD1", EqDef::Fn(id1)),
        ("imi", EqDef::Fn(imi)),
        ("inn", EqDef::Fn(inn)),
        ("p1avg", EqDef::Fn(p1avg)),
        ("f1rescale", EqDef::Fn(f1rescale)),
        // EQUATION_DUMMY( "cEntry1", "" ) and ( "cExit1", "" )
        ("cEntry1", EqDef::Fn(super::dummy)),
        ("cExit1", EqDef::Fn(super::dummy)),
    ]);
}
