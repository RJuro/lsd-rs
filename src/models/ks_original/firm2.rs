//! fun_KS_firm2.h: Firm2 object equations (consumption-good sector firms).
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Prudential maximum bank debt of firm in consumer-good sector. Also updates '_CD2', '_CD2c', '_CS2'.
fn deb2max(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    let cap = capsecl2(c);

    // maximum debt allowed to firm, considering net worth and operating margin
    let m = max(c.vl("_NW2", 1), c.vl("_S2", 1) - c.vl("_W2", 1));
    let v1 = c.vs(fin, "Lambda") * m;

    // apply an absolute floor to maximum debt prudential limit
    let floor = c.vs(fin, "Lambda0") * c.vls(cap, "PPI", 1) / c.vs(cap, "pK0");
    let v0 = max(v1, floor);

    c.write("_CD2", 0.0); // reset firm credit demand
    c.write("_CD2c", 0.0); // reset credit constraint
    c.write("_CS2", 0.0); // reset total credit supplied

    v0
}

/// Dividends to pay by firm in consumption-good sector.
fn div2(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let d = c.vs(par, "d2") * (c.v("_Pi2") - c.v("_Tax2"));
    max(d, 0.0)
}

/// Adaptive demand expectation of firm in consumer-good sector.
fn d2e(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 15];

    let h = c.v("_t2ent") as i32; // firm entry period
    let gp = c.grandparent();
    let k = c.vs(gp, "flagExpect") as i32; // expectation form
    let j = if k == 0 || k > 4 { 1 } else if k == 1 { 4 } else { 2 }; // req. number of data periods

    if h > 0 && h as f64 >= c.t() - 1.0 - j as f64 {
        // entrant or too few data?
        return c.current();
    }

    // compute the mix between fulfilled and potential demand (orders)
    let par = c.parent();
    v[9] = c.vs(par, "e0"); // animal spirits parameter
    for i in 1..=j {
        v[10] = c.vl("_D2", i);
        v[i as usize] = max((1.0 - v[9]) * v[10] + v[9] * c.vl("_D2d", i), v[10]);
    }

    match k {
        // myopic expectations with up to 4-period memory
        1 => {
            v[11] = c.vs(par, "e1"); // weight of t-1 demand
            v[12] = c.vs(par, "e2"); // weight of t-2 demand
            v[13] = c.vs(par, "e3"); // weight of t-3 demand
            v[14] = c.vs(par, "e4"); // weight of t-4 demand

            v[5] = 0.0;
            v[6] = 0.0;
            for i in 1..=4usize {
                if v[i] > 0.0 {
                    // consider only periods with demand
                    v[5] += v[10 + i] * v[i];
                    v[6] += v[10 + i];
                }
            }

            v[0] = if v[6] > 0.0 { v[5] / v[6] } else { 0.0 }; // rescale
        }

        // accelerating GD expectations
        2 => {
            v[2] = max(v[2], 1.0); // floor to positive only

            v[0] = (1.0 + c.vs(par, "e5") * (v[1] - v[2]) / v[2]) * v[1];
        }

        // 1st order adaptive expectations
        3 => {
            v[0] = c.current() + c.vs(par, "e6") * (v[1] - v[2]);
        }

        // extrapolative-accelerating expectations
        4 => {
            v[2] = max(v[2], 1.0); // floor to positive only

            let a = c.vs(par, "e7") * (v[1] - v[2]) / v[2];
            let b = c.vs(par, "e8") * c.vls(gp, "dGDP", 1);
            v[0] = (1.0 + a + b) * v[1];
        }

        // myopic expectations with 1-period memory
        _ => {
            v[0] = v[1];
        }
    }

    v[0]
}

/// Effective competitiveness of a firm in sector 2, considering the price, unfilled demand and the quality of the
/// product for the consumer.
fn e(c: &mut Ctx) -> f64 {
    let par = c.parent();
    if c.v("_life2cycle") == 0.0 {
        return c.vls(par, "Eavg", 1); // non-producing entrant
    }

    let v1 = c.vls(par, "l2avg", 1) + 1.0;

    let a = -c.vs(par, "omega1") * c.v("_p2") / c.vs(par, "p2avg");
    let b = c.vs(par, "omega2") * if v1 > 1.0 { (c.vl("_l2", 1) + 1.0) / v1 } else { 1.0 };
    a - b
}

/// Effective expansion investment of firm in consumption-good sector.
/// Also updates '_NW2', '_Deb2', _CD2', '_CD2c', '_CS2'.
fn ei(c: &mut Ctx) -> f64 {
    c.v("_Q2"); // make sure production decided
    c.v("_supplier"); // ensure supplier is selected
    let d = c.v("_EId");
    let this = c.p;
    invest(c, this, d)
}

/// Desired expansion investment of firm in consumption-good sector.
fn eid(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 8];

    v[1] = c.v("_Kd"); // desired capital
    v[2] = c.vl("_K", 1); // available capital stock
    let par = c.parent();
    v[3] = c.vs(par, "m2"); // machine output per period

    if v[2] < v[3] {
        // no capital yet?
        return v[1]; // no growth threshold
    }

    v[4] = c.vs(par, "kappaMin"); // investment floor multiple

    // min rounded capital
    v[5] = ((1.0 + v[4]) * v[2] / v[3]).round() * v[3];

    if v[1] > v[5] {
        // minimum capital reached?
        if v[4] > 0.0 {
            v[0] = v[5] - v[2];
        } else {
            v[6] = c.vs(par, "kappaMax"); // investment cap multiple
            v[7] = ((1.0 + v[6]) * v[2] / v[3]).round() * v[3];

            if v[6] > 0.0 && v[1] > v[7] {
                v[0] = v[7] - v[2]; // max rounded capital
            } else {
                v[0] = ((v[1] - v[2]) / v[3]).floor() * v[3];
            }
        }
    } else {
        v[0] = 0.0; // no expansion investment
    }

    v[0]
}

/// Desired capital stock of firm in consumption-good sector.
fn kd(c: &mut Ctx) -> f64 {
    if c.v("_life2cycle") == 0.0 {
        // if fresh entrant
        return c.current(); // keep initially desired capital
    }

    // desired capacity with slack and utilization, based on expectations/inventories
    let par = c.parent();
    let x = (1.0 + c.vs(par, "iota")) * c.v("_D2e") - c.vl("_N", 1);
    max(x, 0.0) / c.vs(par, "u")
}

/// Planned production for a firm in consumption-good sector.
/// Also updates '_NW2', '_NW2p', '_Deb2', '_CD2', '_CD2c', '_CS2'.
fn q2(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 10];
    let par = c.parent();

    v[1] = c.v("_Q2d"); // desired production
    v[2] = c.v("_CS2a"); // available credit supply
    v[3] = c.vl("_NW2", 1); // net worth (cash available)
    v[4] = c.v("_c2"); // expected unit cost
    v[5] = c.vs(par, "m2"); // machine output per period

    v[6] = v[1] * v[4]; // cost of desired production

    if v[6] <= v[3] {
        // firm can self-finance?
        v[0] = v[1]; // plan the desired output
        v[7] = v[3] - v[6]; // remove wage cost from cash
        v[8] = 0.0; // no finance
    } else {
        if v[6] <= v[3] + v[2] {
            // possible to finance all?
            v[0] = v[1]; // plan the desired output
            v[7] = 0.0; // no cash
            v[9] = v[6] - v[3]; // finance the difference
            v[8] = v[9];
        } else {
            // credit constrained firm
            // produce as much as the available finance allows, rounded # machines
            v[0] = (max((v[3] + v[2]) / v[4], 0.0) / v[5]).floor() * v[5];
            v[9] = v[6] - v[3]; // desired credit

            if v[0] == 0.0 {
                v[7] = v[3]; // keep cash
                v[8] = 0.0; // no finance
            } else {
                v[6] = v[0] * v[4]; // reduced production cost
                if v[6] <= v[3] {
                    v[7] = v[3] - v[6]; // pay with available cash
                    v[8] = 0.0; // no finance
                } else {
                    v[7] = 0.0; // no cash
                    v[8] = v[6] - v[3]; // finance the difference
                }
            }
        }

        let this = c.p;
        update_debt(c, this, v[9], v[8]); // update debt (desired/granted)
    }

    let this = c.p;
    update_depo(c, this, v[7], false); // update the firm net worth
    c.write("_NW2p", v[3] - v[7] + v[8]); // provision for production

    v[0]
}

/// Desired output of firm in consumption-good sector.
fn q2d(c: &mut Ctx) -> f64 {
    if c.v("_life2cycle") == 0.0 {
        // if fresh entrant
        return 0.0; // not yet producing
    }

    // desired production with slack, based on expectations, considering inventories
    let par = c.parent();
    let x = (1.0 + c.vs(par, "iota")) * c.v("_D2e") - c.vl("_N", 1);
    let v1 = max(x, 0.0);

    // limited to the available capital stock
    min(v1, c.vl("_K", 1))
}

/// Effective substitution investment of firm in consumption-good sector.
/// Also updates '_NW2', '_Deb2', _CD2', '_CD2c', '_CS2'.
fn si(c: &mut Ctx) -> f64 {
    c.v("_EI"); // make sure expansion done
    let d = c.v("_SId");
    let this = c.p;
    invest(c, this, d)
}

/// Tax paid by firm in consumption-good sector.
/// Also updates '_NW2', '_Deb2', _CD2', '_CD2c', '_CS2'.
fn tax2(c: &mut Ctx) -> f64 {
    let v1 = c.v("_Pi2"); // firm profit in period

    let v0 = if v1 > 0.0 {
        // profits?
        let gp = c.grandparent();
        v1 * c.vs(gp, "tr") // tax to government
    } else {
        0.0 // no tax on losses
    };

    let this = c.p;
    cash_flow(c, this, v1, v0); // manage the period cash flow

    v0
}

/// Allocate workers to vintages, prioritizing newer vintages. Also updates '__Lvint'.
fn alloc2(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 3];

    let Some(mut cur) = c.hook(TOPVINT) else { return 0.0 }; // start with top vintage; no vintage, nothing to do

    c.v("_c2"); // ensure machines are set
    v[1] = c.v("_L2"); // workers to allocate
    v[2] = c.vs(cur, "__LdVint"); // labor demand of vintage

    v[0] = 0.0; // allocated worker counter
    'done: while v[1] > 0.0 {
        // allocate all available workers
        while v[2] == 0.0 {
            // find vintage with posit. dem.
            match c.shooks(cur) {
                // so go to the previous one
                None => break 'done,
                Some(prev) => cur = prev,
            }
            if c.vs(cur, "__toUseVint") == 0.0 {
                // oldest or done?
                break 'done; // not possible to allocate more
            }

            v[2] = c.vs(cur, "__LdVint"); // labor demand of vintage
        }

        if v[1] >= v[2] {
            // can fully fulfill demand?
            c.writes(cur, "__Lvint", v[2]); // update available worker counter
            v[1] -= v[2];
            v[0] += v[2];
            v[2] = 0.0;
        } else {
            c.writes(cur, "__Lvint", v[1]); // supply what is available
            v[0] += v[1];
            v[1] = 0.0;
        }
    }

    v[0]
}

/// Planned average unit cost of firm in consumption-good sector.
/// Considers even unused machines, as in original K+S. Also updates '_A2'.
fn c2(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 11];

    v[1] = c.v("_Q2d"); // desired production
    v[2] = c.vl("_K", 1); // available capital stock
    let par = c.parent();
    v[3] = c.vs(par, "m2"); // machine output per period
    let lab = labsupl2(c);
    v[4] = c.vs(lab, "w"); // firm wage

    v[5] = max((v[2] / v[3]).floor() - (v[1] / v[3]).ceil(), 0.0); // unused machines

    // scan all vintages, from oldest to newest, preferring to use newer ones
    v[0] = 0.0;
    v[6] = 0.0;
    v[7] = 0.0; // accumulators
    let mut cur = c.first("Vint");
    while let Some(o) = cur {
        // choose vintages to use
        v[8] = c.vs(o, "__nVint"); // number of machines in vintage
        v[9] = c.vs(o, "__Avint"); // vintage productivity

        v[0] += v[8] * v[4] / v[9]; // add machines cost
        v[6] += v[8]; // add machines
        v[7] += v[8] * v[9]; // add productivities

        if v[5] >= v[8] {
            // none to be used in the vint.?
            v[5] -= v[8]; // less machines not to use
            v[10] = 0.0; // no machines to use in vintage
        } else {
            v[10] = v[8] - v[5]; // machines to use in vintage
            v[5] = 0.0; // no more machine not to use
        }

        c.writes(o, "__toUseVint", v[10]); // number mach. to try to use
        c.writes(o, "__Lvint", 0.0); // no worker allocated yet
        cur = c.next(o);
    }

    if v[6] == 0.0 {
        // no machine?
        c.v("_supplier"); // ensure supplier is selected
        let broch = c.hook(SUPPL).expect("brochure");
        let cur1 = c.up(c.shooks(broch).expect("client entry")); // pointer to supplier

        v[6] = 1.0; // 1 notional machine
        v[7] = c.vs(cur1, "_Atau"); // new machines productivity
        v[0] = v[4] / v[7]; // machine unit cost
    }

    c.write("_A2", v[7] / v[6]); // potential productivity

    v[0] / v[6]
}

/// Market share of firm in consumption-good sector.
/// It is computed using a replicator equation over the relative competitiveness of the firm.
/// Because of entrants, market shares may add-up to more than one, so 'f2rescale'
/// must be used before '_f2' is used, by calling 'CPI'.
fn f2(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 2];

    if c.v("_life2cycle") == 0.0 {
        // entrant firm state
        return 0.0;
    }

    let par = c.parent();
    v[1] = c.vs(par, "f2min"); // minimum share to stay

    if c.current() == 0.0 && c.v("_t2ent") == c.t() - 2.0 {
        // first-period entrant	?
        v[0] = c.vl("_K", 1) / c.vls(par, "K", 1); // same as capital share
        return max(v[0], v[1]); // but over minimum
    }

    // replicator equation
    let x = c.vl("_f2", 1);
    let chi = c.vs(par, "chi");
    let ee = c.v("_E") / c.vs(par, "Eavg") - 1.0;
    v[0] = x * (1.0 - chi * ee);

    // lower-bounded slightly below exit threshold
    // to ensure firm sells production before leaving the market
    max(v[0], 0.99 * v[1])
}

/// Mark-up of firm in consumption-good sector.
fn mu2(c: &mut Ctx) -> f64 {
    let v1 = c.vl("_f2", 1); // past periods market shares
    let v2 = c.vl("_f2", 2);
    let par = c.parent();
    let v3 = c.vs(par, "f2min"); // market exit share threshold

    if v1 < v3 || v2 < v3 {
        // just entered firms keep it
        return c.current();
    }

    c.current() * (1.0 + c.vs(par, "upsilon") * (v1 / v2 - 1.0))
}

/// Selected machine supplier by firm in consumption-good sector.
/// Also set firm 'hook' pointers to supplier firm object.
fn supplier(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 6];

    let cap = capsecl2(c);
    c.vs(cap, "inn"); // ensure innovation is done and brochures distributed
    let par = c.parent();
    v[1] = c.vs(par, "m2"); // machine modularity
    v[2] = c.vs(par, "b"); // required payback period

    v[4] = f64::MAX; // supplier price/cost ratio
    let mut i: i32 = 0;
    let (mut cur2, mut cur3) = (None, None);
    let mut cur = c.first("Broch");
    while let Some(o) = cur {
        // use brochures to find supplier
        let cur1 = c.up(c.shooks(o).expect("client entry")); // pointer to supplier object

        // compare total machine unit cost (acquisition + operation for payback period)
        v[5] = c.vs(cur1, "_p1") / v[1] + c.vs(cur1, "_cTau") * v[2];
        if v[5] < v[4] {
            // best so far?
            v[4] = v[5]; // save current best supplier
            i = c.vs(cur1, "_ID1") as i32; // supplier ID
            cur2 = c.shooks(o); // own entry on supplier list
            cur3 = Some(o); // best supplier brochure
        }
        cur = c.next(o);
    }

    // if supplier is found, simply update it, if not, draw a random one
    if let (Some(cur2), Some(cur3)) = (cur2, cur3) {
        let t = c.t();
        c.writes(cur2, "__tSel", t); // update selection time
        c.write_hook(SUPPL, cur3); // pointer to current brochure
    } else {
        // no brochure received
        let this = c.p;
        let cur1 = set_supplier(c, this); // draw new supplier
        i = c.vs(cur1, "_ID1") as i32;
    }

    i as f64
}

/// Canceled investment of firm in consumption-good sector.
fn ci(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 6];

    c.v("_supplier"); // ensure new supplier is set

    let cur = c.hook(SUPPL).expect("brochure"); // pointer to current supplier
    let cli = c.shooks(cur).expect("client entry");
    let sup = c.up(cli);
    c.vs(sup, "_Q1e"); // make sure supplier produced

    v[1] = c.vs(cli, "__nCan"); // canceled machine number
    let k = c.vs(cli, "__tOrd") as i32; // time of canceled order

    if k as f64 == c.t() && v[1] > 0.0 {
        let par = c.parent();
        v[2] = c.vs(par, "m2"); // machine output per period
        v[3] = c.v("_SI") / v[2]; // machines to substitute
        v[4] = c.v("_EI") / v[2]; // machines to expand
        v[5] = c.vs(sup, "_p1"); // machine price

        if v[1] > v[3] {
            // no space for substitution?
            c.write("_SI", 0.0); // reset substitution investment
            c.write("_EI", (v[4] - v[1] + v[3]) * v[2]); // remaining expansion inv.
        } else {
            c.write("_SI", (v[3] - v[1]) * v[2]); // shrink substitution investm.
        }

        let this = c.p;
        update_depo(c, this, v[5] * v[1], true); // recover paid machines value

        v[0] = v[1] * v[2]; // canceled investment
    } else {
        v[0] = 0.0;
    }

    v[0]
}

/// Desired (potential) demand for firm in consumption-good sector.
fn d2d(c: &mut Ctx) -> f64 {
    let par = c.parent();
    c.vs(par, "CPI"); // ensure m.s. updated
    let x = c.v("_f2");
    x * c.vs(par, "D2d")
}

/// Investment (nominal/currency terms) of firm in consumption-good sector.
fn inom(c: &mut Ctx) -> f64 {
    c.v("_K"); // ensure capital is deployed

    let cur = c.hook(TOPVINT); // last capital vintage

    match cur {
        Some(o) if c.vs(o, "__tVint") == c.t() => {
            // capital deployed in period?
            let x = c.vs(o, "__nVint");
            x * c.vs(o, "__pVint")
        }
        _ => 0.0,
    }
}

/// Open job positions for a firm in consumption-good sector.
fn jo2(c: &mut Ctx) -> f64 {
    let x = c.v("_L2d") - c.vl("_L2", 1);
    max(x, 0.0)
}

/// Capital employed by firm in consumption-good sector.
fn k(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 9];

    c.v("_CI"); // ensure cancel investment done

    let par = c.parent();
    v[1] = c.vs(par, "m2"); // machine output per period
    v[2] = c.v("_SI"); // substitution investment
    v[3] = c.v("_EI"); // expansion investment

    // new machines delivered only if supplier has not canceled the order
    if v[2] + v[3] > 0.0 {
        v[4] = ((v[2] + v[3]) / v[1]).floor(); // total number of new machines

        if v[4] > 0.0 {
            // new machines to install?
            let this = c.p;
            add_vintage(c, this, v[4], false); // create vintage
        }
    }

    v[5] = max(c.vl("_K", 1) + v[3] - c.v("_Kd"), 0.0); // desired capital shrinkage
    v[6] = (v[5] / v[1]).floor(); // machines to remove from K

    v[7] = (v[2] / v[1]).floor(); // machines to substitute in K

    let mut j: i32 = c.t() as i32 + 1; // oldest vintage so far
    let mut h: i32 = 0; // oldest vintage ID
    let mut cur = c.first("Vint");
    while let Some(o) = cur {
        // search from older vintages
        let next = c.next(o); // CYCLE_SAFE
        'body: {
            v[8] = c.vs(o, "__RSvint"); // number of machines to scrap

            if v[8] < 0.0 {
                // end-of-life vintage to scrap?
                v[8] = -v[8]; // absolute vintage size

                if v[6] > 0.0 {
                    // yet capital to shrink?
                    if v[8] > v[6] {
                        // more than needed?
                        v[8] -= v[6]; // just reduce vintage
                        v[6] = 0.0; // shrinkage done
                        c.writes(o, "__nVint", v[8]);
                    } else {
                        // scrap entire vintage
                        if scrap_vintage(c, o) >= 0.0 {
                            // not last vintage?
                            v[6] -= v[8];
                            break 'body; // don't consider for old vint.
                        } else {
                            v[8] = 0.0; // last: nothing else to scrap
                            v[6] -= v[8] - 1.0;
                        }
                    }
                }
            }

            // something yet to scrap and substitution to be done?
            if v[8] > 0.0 && v[7] > 0.0 {
                if v[8] > v[7] {
                    // more than needed?
                    v[8] -= v[7]; // just reduce vintage
                    v[7] = 0.0; // substitution done
                    c.writes(o, "__nVint", v[8]);
                } else {
                    // scrap entire vintage
                    if scrap_vintage(c, o) >= 0.0 {
                        // not last vintage?
                        v[7] -= v[8];
                        break 'body; // don't consider for old vint.
                    }
                }
            }

            let i = c.vs(o, "__tVint") as i32; // time of vintage install
            if i < j {
                // oldest so far?
                j = i;
                h = c.vs(o, "__IDvint") as i32; // save oldest
            }
        }
        cur = next;
    }

    c.write("_oldVint", h as f64);

    c.sum("__nVint") * v[1]
}

/// Current capital (nominal/currency terms) of firm in consumption-good sector.
fn knom(c: &mut Ctx) -> f64 {
    c.v("_K"); // ensure capital is deployed
    c.whtave("__nVint", "__pVint")
}

/// Labor employed by firm in consumption-good sector.
fn l2(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let v1 = c.vs(par, "L2d");
    if v1 > 0.0 {
        let x = c.v("_L2d");
        x * c.vs(par, "L2") / v1
    } else {
        0.0
    }
}

/// Labor demand of firm in consumption-good sector.
fn l2d(c: &mut Ctx) -> f64 {
    if c.v("_life2cycle") > 0.0 {
        let x = c.v("_Q2");
        x / c.v("_A2")
    } else {
        0.0
    }
}

/// Inventories (unsold output) of firm in consumption-good sector.
fn n(c: &mut Ctx) -> f64 {
    let par = c.parent();
    c.vs(par, "D2"); // ensure demand is allocated

    let v0 = c.current() + c.v("_Q2e") - c.v("_D2");

    round_tol(v0, 0.0, 0.001) // avoid rounding errors on zero
}

/// Profit of firm (before taxes) in consumption-good sector.
fn pi2(c: &mut Ctx) -> f64 {
    let x = c.v("_S2") + c.v("_iD2");
    x - c.v("_W2") - c.v("_i2")
}

/// Effective output of firm in consumption-good sector.
fn q2e(c: &mut Ctx) -> f64 {
    // min( V( "_Q2" ), SUM( "__Qvint" ) ): arguments evaluated right to left
    let b = c.sum("__Qvint");
    let a = c.v("_Q2");
    min(a, b)
}

/// Potential production with current machines for a firm in consumption-good sector.
fn q2p(c: &mut Ctx) -> f64 {
    let x = c.sum("__nVint");
    let par = c.parent();
    x * c.vs(par, "m2")
}

/// Expected potential production with existing workers for a firm in consumption-good sector.
fn q2pe(c: &mut Ctx) -> f64 {
    let x = c.vl("_L2", 1);
    x * c.v("_A2")
}

/// Capacity utilization for a firm in consumption-good sector.
fn q2u(c: &mut Ctx) -> f64 {
    let v1 = c.v("_Q2p");
    if v1 > 0.0 {
        c.v("_Q2e") / v1
    } else {
        let par = c.parent();
        c.vls(par, "Q2u", 1)
    }
}

/// Sales of firm in consumption-good sector.
fn s2(c: &mut Ctx) -> f64 {
    let par = c.parent();
    c.vs(par, "D2"); // ensure demand is allocated
    let x = c.v("_p2");
    x * c.v("_D2")
}

/// Desired substitution investment of firm in consumption-good sector.
fn sid(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 6];
    let par = c.parent();

    v[1] = c.vs(par, "m2"); // machine output per period

    v[2] = 0.0; // scrapped machine accumulator
    let mut cur1 = c.first("Vint");
    while let Some(o) = cur1 {
        // search last vintage to scrap
        v[3] = c.vs(o, "__RSvint"); // number of machines to scrap

        if v[3] == 0.0 {
            // nothing else to do
            break;
        }

        v[2] += abs(v[3]); // accumulate vintage
        cur1 = c.next(o);
    }

    v[4] = max(c.vl("_K", 1) - c.v("_Kd"), 0.0); // capital shrinkage desired?
    v[5] = (v[4] / v[1]).floor(); // machines to remove from K

    max(v[2] - v[5], 0.0) * v[1]
}

/// Total wages paid by firm in sector 2.
fn w2(c: &mut Ctx) -> f64 {
    let x = c.v("_L2");
    let lab = labsupl2(c);
    x * c.vs(lab, "w")
}

/// Effective average unit cost of firm in consumption-good sector. Use expected cost if firm is not producing.
fn c2e(c: &mut Ctx) -> f64 {
    let v1 = c.v("_Q2e");
    if v1 > 0.0 { c.v("_W2") / v1 } else { c.v("_c2") }
}

/// Notional productivity (bounded) rate of change of firm in consumption-good sector. Used for wages adjustment only.
fn d_a2b(c: &mut Ctx) -> f64 {
    let gp = c.grandparent();
    let per = c.vs(gp, "mPer");
    let lim = c.vs(gp, "mLim");
    let this = c.p;
    mov_avg_bound(c, this, "_A2", lim, per)
}

/// Change in firm's nominal (currency terms) inventories for a firm in consumption-good sector.
fn d_nnom(c: &mut Ctx) -> f64 {
    let a = c.v("_p2") * c.v("_N");
    a - c.vl("_p2", 1) * c.vl("_N", 1)
}

/// Interest paid by firm in consumption-good sector.
fn i2(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    let x = c.vl("_Deb2", 1);
    x * c.vs(fin, "r")
}

/// Interest received from deposits by firm in consumption-good sector.
fn id2(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    let x = c.vl("_NW2", 1);
    x * c.vs(fin, "r")
}

/// Stage in life cycle of firm in consumer-good sector:
/// 0 = pre-operational entrant firm, 1 = operating entrant firm (first period producing).
fn life2cycle(c: &mut Ctx) -> f64 {
    if c.current() > 0.0 {
        1.0
    } else if c.vl("_K", 1) > 0.0 {
        1.0
    } else {
        0.0
    }
}

/// Price of good of firm in consumption-good sector. Entrants notional price are the market average.
fn p2(c: &mut Ctx) -> f64 {
    let x = 1.0 + c.v("_mu2");
    let par = c.parent();
    x * if c.v("_life2cycle") == 0.0 { c.vls(par, "c2", 1) } else { c.v("_c2") }
}

/// Bank credit supply available (new debt) to firm in consumer-good sector. Function called multiple times in single time step.
fn cs2a(c: &mut Ctx) -> f64 {
    let x = c.v("_Deb2max") - c.v("_Deb2");
    max(x, 0.0)
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("_Deb2max", EqDef::Fn(deb2max)),
        ("_Div2", EqDef::Fn(div2)),
        ("_D2e", EqDef::Fn(d2e)),
        ("_E", EqDef::Fn(e)),
        ("_EI", EqDef::Fn(ei)),
        ("_EId", EqDef::Fn(eid)),
        ("_Kd", EqDef::Fn(kd)),
        ("_Q2", EqDef::Fn(q2)),
        ("_Q2d", EqDef::Fn(q2d)),
        ("_SI", EqDef::Fn(si)),
        ("_Tax2", EqDef::Fn(tax2)),
        ("_alloc2", EqDef::Fn(alloc2)),
        ("_c2", EqDef::Fn(c2)),
        ("_f2", EqDef::Fn(f2)),
        ("_mu2", EqDef::Fn(mu2)),
        ("_supplier", EqDef::Fn(supplier)),
        ("_CI", EqDef::Fn(ci)),
        ("_D2d", EqDef::Fn(d2d)),
        ("_Inom", EqDef::Fn(inom)),
        ("_JO2", EqDef::Fn(jo2)),
        ("_K", EqDef::Fn(k)),
        ("_Knom", EqDef::Fn(knom)),
        ("_L2", EqDef::Fn(l2)),
        ("_L2d", EqDef::Fn(l2d)),
        ("_N", EqDef::Fn(n)),
        ("_Pi2", EqDef::Fn(pi2)),
        ("_Q2e", EqDef::Fn(q2e)),
        ("_Q2p", EqDef::Fn(q2p)),
        ("_Q2pe", EqDef::Fn(q2pe)),
        ("_Q2u", EqDef::Fn(q2u)),
        ("_S2", EqDef::Fn(s2)),
        ("_SId", EqDef::Fn(sid)),
        ("_W2", EqDef::Fn(w2)),
        ("_c2e", EqDef::Fn(c2e)),
        ("_dA2b", EqDef::Fn(d_a2b)),
        ("_dNnom", EqDef::Fn(d_nnom)),
        ("_i2", EqDef::Fn(i2)),
        ("_iD2", EqDef::Fn(id2)),
        ("_life2cycle", EqDef::Fn(life2cycle)),
        ("_p2", EqDef::Fn(p2)),
        ("_CS2a", EqDef::Fn(cs2a)),
        // dummy equations (the value is written by other equations)
        ("_A2", EqDef::Fn(super::dummy)),
        ("_CD2", EqDef::Fn(super::dummy)),
        ("_CD2c", EqDef::Fn(super::dummy)),
        ("_CS2", EqDef::Fn(super::dummy)),
        ("_D2", EqDef::Fn(super::dummy)),
        ("_Deb2", EqDef::Fn(super::dummy)),
        ("_NW2", EqDef::Fn(super::dummy)),
        ("_NW2p", EqDef::Dummy("_Q2")),
        ("_l2", EqDef::Fn(super::dummy)),
        ("_oldVint", EqDef::Dummy("_K")),
    ]);
}
