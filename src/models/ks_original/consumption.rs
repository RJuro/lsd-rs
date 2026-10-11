//! fun_KS_consumption.h: Consumer-goods market object equations.
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Demand fulfilled by firms in consumption-good sector. Update '_D2', '_l2'.
fn d2(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 6];

    let k = c.v("F2") as i32; // number of firms
    let par = c.parent();
    v[1] = c.vs(par, "Cd"); // nominal demand

    // create and fill temporary share and supply vectors & initialize firm demand
    let n = k as usize;
    let (mut f2, mut p2, mut sup2) = (vec![0.0f64; n], vec![0.0f64; n], vec![0.0f64; n]);

    let mut j: usize = 0;
    let mut cur = c.first("Firm2");
    while let Some(o) = cur {
        sup2[j] = c.vs(o, "_Q2e") + c.vls(o, "_N", 1); // firm available supply
        f2[j] = c.vs(o, "_f2"); // firm market share
        p2[j] = c.vs(o, "_p2"); // firm price
        c.writes(o, "_D2", 0.0); // demand fulfilled accumulator
        c.writes(o, "_l2", 0.0); // assume no unsatisfied demand
        j += 1;
        cur = c.next(o);
    }

    // cycle through firms until all demand is allocated or no more product to sell
    v[0] = 0.0; // fulfilled demand accumulator
    let mut i = 0;
    while v[1] > 0.01 {
        v[2] = v[1]; // remaining unallocated $ demand
        v[3] = 0.0; // shares yet unallocated
        j = 0;
        let mut cur = c.first("Firm2");
        while let Some(o) = cur {
            if f2[j] > 0.0 {
                // firm has demand to supply
                if sup2[j] > 0.0 {
                    // product to supply?
                    v[4] = v[1] * f2[j]; // firm $ demand allocation
                    v[5] = v[4] / p2[j]; // firm # demand allocation

                    if v[5] <= sup2[j] {
                        // can supply all demanded?
                        c.incrs(o, "_D2", v[5]); // supply all demanded
                        v[0] += v[5]; // accumulate to total # demand
                        v[2] -= v[4]; // discount from desired $ demand
                        v[3] += f2[j]; // save share yet to allocate
                        sup2[j] -= v[5]; // make supplied # unavailable
                    } else {
                        if i == 0 {
                            // unsatisfied demand metric
                            c.writes(o, "_l2", v[5] - sup2[j]);
                        }

                        c.incrs(o, "_D2", sup2[j]); // supply all # available
                        v[0] += sup2[j]; // accumulate to total # demand
                        v[2] -= sup2[j] * p2[j]; // discount from desired $ demand
                        sup2[j] = 0.0; // nothing else to supply
                        f2[j] = 0.0;
                    }
                } else {
                    f2[j] = 0.0; // nothing else to supply
                }
            }

            j += 1;
            cur = c.next(o);
        }

        if v[3] > 0.0 {
            // unallocated shares remaining?
            for jj in 0..n {
                f2[jj] /= v[3]; // rescale remaining firms
            }
        } else {
            break; // nothing else to supply
        }

        v[1] = v[2]; // update unallocated
        i += 1;
    }

    v[0]
}

/// Rate of entry-exit of firms in consumer-good sector.
/// Perform entry and exit of firms in the consumer-good sector.
/// All relevant aggregate variables in sector must be computed before existing
/// firms are deleted, so all active firms in period are considered.
/// Also updates 'cEntry2', 'cExit2', 'exit2fail'.
fn entry2exit(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 8];

    c.sum("_D2d"); // desired demand before chg
    c.update(); // ensure aggregates are computed

    let cap = capsecl1(c);
    let nw20u = c.v("NW20") * c.vs(cap, "PPI") / c.vs(cap, "pK0"); // minimum wealth in sector 2
    let f2min = c.v("f2min"); // min market share in sector 2
    let n2 = c.v("n2"); // market participation period
    let f2 = c.v("F2") as i32; // current number of firms

    let mut quit = vec![false; f2 as usize]; // vector of firms' quit status

    c.write("cEntry2", 0.0); // reset exit/entry accumulators
    c.write("cExit2", 0.0);

    // mark bankrupt and market-share-irrelevant incumbent firms to quit the market
    let mut h = f2; // initial number of firms
    v[1] = 0.0;
    v[3] = 0.0;
    let mut i: i32 = 0;
    let mut k: i32 = 0; // accum., counters, registers
    let mut cur = c.first("Firm2");
    while let Some(o) = cur {
        v[4] = c.vs(o, "_NW2"); // current net wealth

        if v[4] < 0.0 || c.vs(o, "_life2cycle") > 0.0 {
            // bankrupt or incumbent?
            v[5] = 0.0;
            let mut j: i32 = 0;
            while (j as f64) < n2 {
                v[5] += c.vls(o, "_f2", j) / n2; // n2 periods avg. market share
                j += 1;
            }

            if v[4] < 0.0 || v[5] < f2min {
                quit[i as usize] = true; // mark for likely exit
                h -= 1; // one less firm

                if v[5] > v[3] {
                    // best firm so far?
                    k = i; // save firm index
                    v[3] = v[5]; // and market share
                }
            }
        }

        i += 1;
        cur = c.next(o);
    }

    // quit candidate firms exit, except the best one if all going to quit
    v[6] = 0.0;
    i = 0;
    let mut j: i32 = 0; // firm counters
    let mut cur = c.first("Firm2");
    while let Some(o) = cur {
        let next = c.next(o); // CYCLE_SAFE
        if quit[i as usize] {
            if h > 0 || i != k {
                // firm must exit?
                j += 1; // count exits
                if c.vs(o, "_NW2") < 0.0 {
                    // count bankruptcies
                    v[6] += 1.0;
                }

                exit_firm(c, o); // del obj & collect liq. val.
            } else if h == 0 && i == k {
                // best firm must get new equity
                // new equity required
                v[7] = nw20u + c.vs(o, "_Deb2") - c.vs(o, "_NW2");
                v[1] += v[7];

                c.writes(o, "_Deb2", 0.0); // reset debt
                c.incrs(o, "_Eq2", v[7]); // add new equity
                c.incrs(o, "_NW2", v[7]);
            }
        }

        i += 1;
        cur = next;
    }

    c.v("f2rescale"); // redistribute exiting m.s.

    // replace exiting firms by entrants
    let p = c.p;
    entry_firm2(c, p, j, false); // add entrant-firm objects

    c.incr("cEntry2", v[1]); // add cost of additional equity
    let sec = secstal1(c);
    c.writes(sec, "exit2fail", v[6] / f2 as f64);

    c.v("f2rescale"); // redistribute entrant m.s.
    c.v("firm2maps"); // update firm mapping vectors

    j as f64 / f2 as f64
}

/// Machine-level weighted-average labor productivity of consumption-good sector.
fn a2(c: &mut Ctx) -> f64 {
    c.v("CPI"); // ensure m.s. are updated
    c.whtave("_A2", "_f2")
}

/// Total canceled investment in consumption-good sector.
fn ci(c: &mut Ctx) -> f64 {
    c.sum("_CI")
}

/// Consumer price index.
fn cpi(c: &mut Ctx) -> f64 {
    c.v("f2rescale"); // ensure m.s. computed/rescaled
    c.whtave("_p2", "_f2")
}

/// Desired demand for firms in consumption-good sector.
fn d2d(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let x = c.vs(par, "Cd");
    x / c.v("CPI")
}

/// Demand expectation of firms in consumer-good sector.
fn d2e(c: &mut Ctx) -> f64 {
    c.sum("_D2e")
}

/// Total debt of consumption-good sector.
fn deb2(c: &mut Ctx) -> f64 {
    c.sum("_Deb2")
}

/// Total dividends paid by firms in consumption-good sector.
fn div2(c: &mut Ctx) -> f64 {
    c.v("Tax2"); // ensure dividends are computed
    c.sum("_Div2")
}

/// Total expansion investment in consumption-good sector.
fn ei(c: &mut Ctx) -> f64 {
    c.v("CI"); // ensure cancellations acct'd
    c.sum("_EI")
}

/// Weighted average competitiveness of firms in consumption-good sector.
fn eavg(c: &mut Ctx) -> f64 {
    let mut v0 = 0.0;
    let mut cur = c.first("Firm2");
    while let Some(o) = cur {
        if c.vs(o, "_life2cycle") > 0.0 {
            // consider only non-entrants
            v0 += c.vs(o, "_E") * c.vls(o, "_f2", 1);
        }
        // compute weighted average
        cur = c.next(o);
    }
    v0
}

/// Equity hold by workers/households from firms in consumption-good sector.
fn eq2(c: &mut Ctx) -> f64 {
    c.sum("_Eq2")
}

/// Total desired investment in terms of output capacity (real terms).
/// Don't recompute 'SI'/'EI' at this stage, to wait for order cancellations.
fn id(c: &mut Ctx) -> f64 {
    let x = c.sum("_SI");
    x + c.sum("_EI")
}

/// Aggregated investment (nominal/currency terms).
fn inom(c: &mut Ctx) -> f64 {
    c.sum("_Inom")
}

/// Aggregated real investment (in initial prices terms).
fn ireal(c: &mut Ctx) -> f64 {
    let x = c.v("SI") + c.v("EI");
    let cap = capsecl1(c);
    x / c.v("m2") * c.vs(cap, "pK0")
}

/// Open job positions in consumption-good sector.
fn jo2(c: &mut Ctx) -> f64 {
    c.sum("_JO2")
}

/// Total capital accumulated by firms in consumption-good sector. After new machine orders are delivered.
fn k(c: &mut Ctx) -> f64 {
    c.sum("_K")
}

/// Total desired capital stock of firms in consumption-good sector.
fn kd(c: &mut Ctx) -> f64 {
    c.sum("_Kd")
}

/// Total capital (nominal/money terms) in consumption-good sector.
fn knom(c: &mut Ctx) -> f64 {
    c.sum("_Knom")
}

/// Work force (labor) size in consumption-good sector.
fn l2(c: &mut Ctx) -> f64 {
    let lab = labsupl1(c);
    let cap = capsecl1(c);
    let v1 = c.vs(lab, "Ls") - c.vs(cap, "L1"); // available labor force
    let v2 = c.v("L2d"); // desired workers in sector 2
    min(v2, v1) // pick up to available
}

/// Total labor demand from firms in consumption-good sector.
fn l2d(c: &mut Ctx) -> f64 {
    c.sum("_L2d")
}

/// Total inventories (real terms).
fn n(c: &mut Ctx) -> f64 {
    c.sum("_N")
}

/// Total net wealth (free cash) of firms in consumption-good sector.
fn nw2(c: &mut Ctx) -> f64 {
    c.sum("_NW2")
}

/// Total profits of consumer-good sector.
fn pi2(c: &mut Ctx) -> f64 {
    c.sum("_Pi2")
}

/// Total planned output before labor/financial constraints in consumption-good sector.
fn q2(c: &mut Ctx) -> f64 {
    c.sum("_Q2")
}

/// Total desired real output of consumption-good sector.
fn q2d(c: &mut Ctx) -> f64 {
    c.sum("_Q2d")
}

/// Total effective output of firms in consumption-good sector.
fn q2e(c: &mut Ctx) -> f64 {
    c.sum("_Q2e")
}

/// Potential production with current machines in consumption-good sector.
fn q2p(c: &mut Ctx) -> f64 {
    c.sum("_Q2p")
}

/// Capacity utilization of consumption-good sector.
fn q2u(c: &mut Ctx) -> f64 {
    let v1 = c.v("Q2p");
    if v1 > 0.0 { c.v("Q2e") / v1 } else { 0.0 }
}

/// Total sales of consumption-good sector.
fn s2(c: &mut Ctx) -> f64 {
    c.sum("_S2")
}

/// Total substitution investment in consumption-good sector.
fn si(c: &mut Ctx) -> f64 {
    c.v("CI"); // ensure cancellations acct'd
    c.sum("_SI")
}

/// Total taxes paid by firms in consumption-good sector.
fn tax2(c: &mut Ctx) -> f64 {
    c.sum("_Tax2")
}

/// Total wages paid by all firms in sector 2.
fn w2(c: &mut Ctx) -> f64 {
    c.sum("_W2")
}

/// Planned average unit cost in consumption-good sector.
fn c2(c: &mut Ctx) -> f64 {
    let v1 = c.v("Q2");
    if v1 > 0.0 { c.whtave("_c2", "_Q2") / v1 } else { c.current() }
}

/// Planned average unit cost in consumption-good sector.
fn c2e(c: &mut Ctx) -> f64 {
    let v1 = c.v("Q2e");
    if v1 > 0.0 { c.whtave("_c2e", "_Q2e") / v1 } else { c.current() }
}

/// Consumer price index inflation (change) rate.
fn d_cpi(c: &mut Ctx) -> f64 {
    let x = c.v("CPI");
    x / c.vl("CPI", 1) - 1.0
}

/// Consumer price index inflation (change) rate.
fn d_cpib(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let per = c.vs(par, "mPer");
    let lim = c.vs(par, "mLim");
    let p = c.p;
    mov_avg_bound(c, p, "CPI", lim, per)
}

/// Change in total nominal inventories (currency terms).
fn d_nnom(c: &mut Ctx) -> f64 {
    c.sum("_dNnom")
}

/// Interest paid by consumption-good sector.
fn i2(c: &mut Ctx) -> f64 {
    c.sum("_i2")
}

/// Interest received from deposits by consumption-good sector.
fn id2(c: &mut Ctx) -> f64 {
    c.sum("_iD2")
}

/// Average unfilled demand in consumption-good sector.
fn l2avg(c: &mut Ctx) -> f64 {
    let v1 = c.sum("_life2cycle");
    if v1 > 0.0 { c.whtave("_l2", "_life2cycle") / v1 } else { c.current() }
}

/// Oldest vintage (ID) in use in period by any firm in consumption-good sector.
fn old_vint(c: &mut Ctx) -> f64 {
    c.min("_oldVint")
}

/// Average price charged in consumption-good sector.
fn p2avg(c: &mut Ctx) -> f64 {
    let v1 = c.sum("_life2cycle");
    if v1 > 0.0 { c.whtave("_p2", "_life2cycle") / v1 } else { c.current() }
}

/// Rescale market shares in consumption-good sector to ensure adding to 1.
/// To be called after market shares are changed in '_f2' and 'entry2exit'.
fn f2rescale(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 3];

    v[1] = c.sum("_f2"); // add-up market shares

    if round_tol(v[1], 1.0, 0.001) == 1.0 {
        // ignore rounding errors
        return v[1];
    }

    v[0] = 0.0; // accumulator

    if v[1] > 0.0 {
        // production ok?
        let mut cur = c.first("Firm2");
        while let Some(o) = cur {
            // rescale to add-up to 1
            v[2] = c.vs(o, "_f2") / v[1]; // rescaled market share
            v[0] += v[2];
            c.writes(o, "_f2", v[2]); // save updated m.s.
            cur = c.next(o);
        }
    } else {
        v[2] = 1.0 / c.count("Firm2"); // firm fair share

        let mut cur = c.first("Firm2");
        while let Some(o) = cur {
            // rescale to add-up to 1
            v[0] += v[2];
            c.writes(o, "_f2", v[2]);
            cur = c.next(o);
        }
    }

    v[0]
}

/// Updates the static maps of firms in consumption-good sector.
/// Only to be called if firm objects in sector 2 are created or destroyed.
fn firm2maps(c: &mut Ctx) -> f64 {
    let par = c.parent();

    // clear vectors
    c.ext_mut::<CountryE>(par).firm2map.clear();
    c.ext_mut::<CountryE>(par).firm2ptr.clear();

    let mut i = 0; // firm index in vector
    let mut cur = c.first("Firm2");
    while let Some(o) = cur {
        // do for all firms in sector 2
        c.ext_mut::<CountryE>(par).firm2ptr.push(o); // pointer to firm
        let id = c.vs(o, "_ID2") as i32;
        c.ext_mut::<CountryE>(par).firm2map.entry(id).or_insert(o); // save in firm's map (insert keeps the first)

        i += 1;
        cur = c.next(o);
    }

    i as f64
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("D2", EqDef::Fn(d2)),
        ("entry2exit", EqDef::Fn(entry2exit)),
        ("A2", EqDef::Fn(a2)),
        ("CI", EqDef::Fn(ci)),
        ("CPI", EqDef::Fn(cpi)),
        ("D2d", EqDef::Fn(d2d)),
        ("D2e", EqDef::Fn(d2e)),
        ("Deb2", EqDef::Fn(deb2)),
        ("Div2", EqDef::Fn(div2)),
        ("EI", EqDef::Fn(ei)),
        ("Eavg", EqDef::Fn(eavg)),
        ("Eq2", EqDef::Fn(eq2)),
        ("Id", EqDef::Fn(id)),
        ("Inom", EqDef::Fn(inom)),
        ("Ireal", EqDef::Fn(ireal)),
        ("JO2", EqDef::Fn(jo2)),
        ("K", EqDef::Fn(k)),
        ("Kd", EqDef::Fn(kd)),
        ("Knom", EqDef::Fn(knom)),
        ("L2", EqDef::Fn(l2)),
        ("L2d", EqDef::Fn(l2d)),
        ("N", EqDef::Fn(n)),
        ("NW2", EqDef::Fn(nw2)),
        ("Pi2", EqDef::Fn(pi2)),
        ("Q2", EqDef::Fn(q2)),
        ("Q2d", EqDef::Fn(q2d)),
        ("Q2e", EqDef::Fn(q2e)),
        ("Q2p", EqDef::Fn(q2p)),
        ("Q2u", EqDef::Fn(q2u)),
        ("S2", EqDef::Fn(s2)),
        ("SI", EqDef::Fn(si)),
        ("Tax2", EqDef::Fn(tax2)),
        ("W2", EqDef::Fn(w2)),
        ("c2", EqDef::Fn(c2)),
        ("c2e", EqDef::Fn(c2e)),
        ("dCPI", EqDef::Fn(d_cpi)),
        ("dCPIb", EqDef::Fn(d_cpib)),
        ("dNnom", EqDef::Fn(d_nnom)),
        ("i2", EqDef::Fn(i2)),
        ("iD2", EqDef::Fn(id2)),
        ("l2avg", EqDef::Fn(l2avg)),
        ("oldVint", EqDef::Fn(old_vint)),
        ("p2avg", EqDef::Fn(p2avg)),
        ("f2rescale", EqDef::Fn(f2rescale)),
        ("firm2maps", EqDef::Fn(firm2maps)),
        // EQUATION_DUMMY( "cEntry2", "" ) and ( "cExit2", "" )
        ("cEntry2", EqDef::Fn(super::dummy)),
        ("cExit2", EqDef::Fn(super::dummy)),
    ]);
}
