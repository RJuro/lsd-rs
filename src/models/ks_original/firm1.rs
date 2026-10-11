//! fun_KS_firm1.h: Firm1 object equations (capital-good sector firms).
use crate::model::{Ctx, EqDef};

use super::class::*;
use super::support::*;

type Reg = Vec<(&'static str, EqDef)>;

/// Labor productivity of new vintage of machine when employed for production. Also updates '_Btau'.
fn atau(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 8];
    let (mut ainn, mut binn, mut aimi, mut bimi) = (0.0, 0.0, 0.0, 0.0);
    // assume innov./imit. failure
    let (mut c_imi, mut c_inn, mut p_imi, mut p_inn) = (f64::MAX, f64::MAX, f64::MAX, f64::MAX);

    let mut atau = c.current(); // previous period productivity
    let mut btau = c.vl("_Btau", 1);
    let con = consecl2(c);
    let b = c.vs(con, "b"); // payback period
    let mut c_tau = c.vl("_cTau", 1); // cur. op. cost. w/ exist. tech.
    let par = c.parent();
    let m1 = c.vs(par, "m1"); // modularity in sector 1
    let m2 = c.vs(con, "m2"); // machine modularity in sector 2
    let mu1 = c.vs(par, "mu1"); // mark-up in sector 1
    let mut p_tau = c.vl("_p1", 1); // current price w/ exist. tech.
    let lab = labsupl2(c);
    let w = c.vs(lab, "w"); // current wage
    let xi = c.vs(par, "xi"); // share of R&D for innovation

    // normalized workers on R&D of the firm
    let l1rd_n = c.vl("_L1rd", 1) * c.vs(lab, "Ls0") / c.vls(lab, "Ls", 1);

    // innovation process (success probability)
    v[1] = 1.0 - (-c.vs(par, "zeta1") * xi * l1rd_n).exp();

    if c.bernoulli(v[1]) != 0.0 {
        // innovation succeeded?
        let x1inf = c.vs(par, "x1inf"); // lower beta inno. draw support
        let x1sup = c.vs(par, "x1sup"); // upper beta inno. draw support
        let alpha1 = c.vs(par, "alpha1"); // beta distrib. alpha parameter
        let beta1 = c.vs(par, "beta1"); // beta distrib. beta parameter

        // new final productivity (A) from innovation
        ainn = atau * (1.0 + x1inf + c.beta(alpha1, beta1) * (x1sup - x1inf));

        // new production productivity (B) from innovation
        binn = btau * (1.0 + x1inf + c.beta(alpha1, beta1) * (x1sup - x1inf));

        // price and operating cost of new machine
        p_inn = (1.0 + mu1) * w / binn / m1;
        c_inn = w / ainn;
    }

    // imitation process (success probability)
    v[2] = 1.0 - (-c.vs(par, "zeta2") * (1.0 - xi) * l1rd_n).exp();

    if c.bernoulli(v[2]) != 0.0 {
        // imitation succeeded?
        let c2avg = c.vls(con, "c2", 1); // average machine operating cost
        let p1avg = c.vls(par, "p1avg", 1); // average machine cost

        let k = c.vs(par, "F1") as i32; // number of firms in sector 1
        let mut imi_prob = vec![0.0f64; k as usize]; // vector for tech distance

        v[3] = 0.0;
        let mut i: usize = 0; // inverse distance/firm accum.
        let this = c.p;
        let mut cur = c.firsts(par, "Firm1");
        while let Some(o) = cur {
            // 1st run: abs. inv. distance
            if o == this {
                imi_prob[i] = 0.0; // can't self-imitate
                i += 1;
            } else {
                // price and operating cost of firm candidate for imitation
                let p = (1.0 + mu1) * w / c.vls(o, "_Btau", 1) / m1;
                let cc = w / c.vls(o, "_Atau", 1);

                // euclidian distance in 2-dimensional mean-standardized space
                let d1 = (p - p_tau) / p1avg;
                let d2 = (cc - c_tau) / c2avg;
                v[4] = (d1 * d1 + d2 * d2).sqrt();

                imi_prob[i] = if v[4] > 0.0 { 1.0 / v[4] } else { 0.0 };
                v[3] += imi_prob[i];
                i += 1;
            }
            cur = c.next(o);
        }

        if v[3] > 0.0 {
            v[5] = 0.0;
            let mut i: usize = 0; // probabilities/firm accum.
            let mut cur = c.firsts(par, "Firm1");
            while let Some(o) = cur {
                // 2nd run: cumulative imi. prob.
                v[5] += imi_prob[i] / v[3]; // normalize to add up to 1
                imi_prob[i] = v[5];
                i += 1;
                cur = c.next(o);
            }

            // draw a firm to imitate according to the distance probabilities
            let r = c.rnd();
            let j = imi_prob.partition_point(|&x| x <= r) as i32; // upper_bound

            if j < k {
                // at least one firm reachable?
                let cur = c.tsearchs(par, "Firm1", (j + 1) as f64); // get pointer to firm

                aimi = c.vls(cur, "_Atau", 1); // get imitated firm productivities
                bimi = c.vls(cur, "_Btau", 1);

                // price and operating cost of new machine
                p_imi = (1.0 + mu1) * w / bimi / m1;
                c_imi = w / aimi;
            }
        }
    }

    // select best option between the three options (current/innovation/imitation)
    v[6] = 0.0;
    v[7] = 0.0; // innovation/imitation flags

    // is innovation ownership unit cost lower than current tech for client?
    if p_inn / m2 + c_inn * b < p_tau / m2 + c_tau * b {
        p_tau = p_inn; // use it
        c_tau = c_inn;
        atau = ainn;
        btau = binn;
        v[6] = 1.0; // innovation succeeded
    }

    // is imitation ownership unit cost even lower?
    if p_imi / m2 + c_imi * b < p_tau / m2 + c_tau * b {
        atau = aimi;
        btau = bimi;
        v[6] = 0.0; // no innovation
        v[7] = 1.0; // imitation succeeded
    }

    c.write("_Btau", btau);
    c.write("_inn", v[6]);
    c.write("_imi", v[7]);

    atau
}

/// Prudential maximum bank debt of firm in capital-good sector. Also updates '_CD1', '_CD1c', '_CS1'.
fn deb1max(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    let par = c.parent();

    // maximum debt allowed to firm, considering net worth and operating margin
    let m = max(c.vl("_NW1", 1), c.vl("_S1", 1) - c.vl("_W1", 1));
    let v5 = c.vs(fin, "Lambda") * m;

    // apply an absolute floor to maximum debt prudential limit
    let floor = c.vs(fin, "Lambda0") * c.vls(par, "PPI", 1) / c.vs(par, "pK0");
    let v0 = max(v5, floor);

    c.write("_CD1", 0.0); // reset total credit demand
    c.write("_CD1c", 0.0); // reset constraint for period
    c.write("_CS1", 0.0); // reset total credit supplied

    v0
}

/// Dividends to pay by firm in capital-good sector.
fn div1(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let d = c.vs(par, "d1") * (c.v("_Pi1") - c.v("_Tax1"));
    max(d, 0.0)
}

/// Number of new client firms in the period. Also creates the client-supplier connecting objects.
fn nc(c: &mut Ctx) -> f64 {
    let gp = c.grandparent();
    let mut firms = c.ext::<CountryE>(gp).firm2map.clone(); // list with all firms

    let k = firms.len() as i32; // number of firms in sector 2
    let h = c.v("_HC") as i32; // number of historical clients

    let mut cur = c.first("Cli");
    while let Some(o) = cur {
        // remove historical clients
        let id = c.vs(o, "__IDc") as i32;
        firms.remove(&id);
        cur = c.next(o);
    }

    let par = c.parent();
    let mut i = (c.vs(par, "gamma") * h as f64).ceil() as i32; // new clients in period
    // i = min( max( i, 1 ), min( k - h, firms.size( ) ) ): max<int>, then LSD's min( double, double )
    let ii = if i > 1 { i } else { 1 };
    i = min(ii as f64, min((k - h) as f64, firms.len() as f64)) as i32; // in [1, F2 - HC]

    let f1 = c.vs(par, "F1");
    let j = max(1.0, (k as f64 / f1).ceil()) as i32; // firm fair share

    if h + i < j {
        // ensure at least fair share
        i = j - h;
    }

    // build vector of all target firms (not yet clients)
    let mut targets: Vec<(i32, ObjIdT)> = firms.iter().map(|(a, b)| (*a, *b)).collect();

    // draw new clients from target list, updating the list after each draw
    let this = c.p;
    let mut k = 0;
    while i > 0 && !targets.is_empty() {
        let h = c.uniform_int(0.0, (targets.len() - 1) as f64) as usize; // draw a index in the list
        let client = targets[h]; // retrieve drawn map pair
        targets.remove(h); // remove drawn firm from list

        // create the brochure/client interconnected objects
        send_brochure(c, this, client.1);
        i -= 1;
        k += 1;
    }

    k as f64
}
type ObjIdT = crate::model::ObjId;

/// Planed production for a firm in capital-good sector. Also updates '_NW1', '_NW1p', '_Deb1', '_CD1', '_CD1c', '_CS1'.
fn q1(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 12];

    v[1] = c.v("_D1"); // potential production (orders)
    v[2] = c.v("_CS1a"); // available credit supply
    v[3] = c.vl("_NW1", 1); // net worth (cash available)
    v[4] = c.v("_c1"); // unit cost
    v[5] = c.v("_p1"); // machine price
    v[6] = c.v("_RD"); // R&D costs still to pay

    v[7] = v[1] * (v[4] - v[5]) + v[6]; // cash to fulfill orders & R&D

    if v[7] < 0.0 || v[7] <= v[3] {
        // firm can self-finance?
        v[0] = v[1]; // plan the desired output
        v[8] = v[3] - v[7]; // remove wage cost from cash
        v[9] = 0.0; // no finance
    } else {
        if v[7] <= v[3] + v[2] {
            // possible to finance all?
            v[0] = v[1]; // plan the desired output
            v[8] = 0.0; // no cash
            v[10] = v[7] - v[3]; // finance the difference
            v[9] = v[10];
        } else {
            // credit constrained firm
            // produce as much as the available finance allows, positive but up to D1
            v[0] = min(max(((v[3] + v[2] - v[6]) / v[4]).floor(), 0.0), v[1]);
            v[10] = v[7] - v[3]; // desired credit

            if v[0] == 0.0 {
                if v[6] <= v[3] {
                    // can pay R&D?
                    v[8] = v[3] - v[6]; // remove R&D wage cost from cash
                    v[10] = 0.0; // no finance
                    v[9] = 0.0;
                } else {
                    // always finance at least R&D
                    v[8] = 0.0; // no cash
                    v[9] = v[6] - v[3]; // always finance at least R&D
                }

                v[11] = 1.0; // all orders canceled
            } else {
                v[7] = v[0] * (v[4] - v[5]) + v[6]; // reduced operation cost
                if v[7] <= v[3] {
                    v[8] = v[3] - v[7]; // pay with available cash
                    v[9] = 0.0; // no finance
                } else {
                    v[8] = 0.0; // no cash
                    v[9] = v[7] - v[3]; // finance the difference
                }

                v[11] = 1.0 - v[0] / v[1]; // machine shortage factor
            }

            // shrink or cancel all exceeding orders
            let t = c.t();
            let mut cur = c.first("Cli");
            while let Some(o) = cur {
                if c.vs(o, "__tOrd") == t {
                    // order in this period?
                    let n = (c.vs(o, "__nOrd") * v[11]).floor();
                    c.incrs(o, "__nCan", n);
                }
                cur = c.next(o);
            }
        }

        let this = c.p;
        update_debt(c, this, v[10], v[9]); // update debt (desired/granted)
    }

    let this = c.p;
    update_depo(c, this, v[8], false); // update the firm net worth
    c.write("_NW1p", v[3] - v[8] + v[9]); // provision for production

    v[0]
}

/// R&D expenditure of firm in capital-good sector.
fn rd(c: &mut Ctx) -> f64 {
    let v1 = c.vl("_S1", 1); // sales in previous period
    let par = c.parent();
    let v2 = c.vs(par, "nu"); // R&D share of sales

    if v1 > 0.0 {
        v2 * v1
    } else {
        // no sales
        // keep current expenditure or a share of available cash
        let cur = c.current();
        min(cur, v2 * c.vl("_NW1", 1))
    }
}

/// Total tax paid by firm in capital-good sector. Also updates '_NW1', '_Deb1', '_CD1', '_CD1c', '_CS1'.
fn tax1(c: &mut Ctx) -> f64 {
    let v1 = c.v("_Pi1"); // firm profit in period

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

/// Price of good of firm in capital-good sector.
fn p1(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let x = 1.0 + c.vs(par, "mu1");
    x * c.v("_c1")
}

/// Number of buying clients for firm in capital-good sector.
fn bc(c: &mut Ctx) -> f64 {
    let t = c.t();
    c.count_cnd("Cli", "__tOrd", "==", t)
}

/// Potential demand (orders) received by a firm in capital-good sector.
fn d1(c: &mut Ctx) -> f64 {
    let con = consecl2(c);
    c.vs(con, "Id"); // make sure all orders are sent
    let t = c.t();
    c.sum_cnd("__nOrd", "__tOrd", "==", t)
}

/// Number of historical client firms from consumer-good sector. Also removes old, non-buying clients.
fn hc(c: &mut Ctx) -> f64 {
    let mut i = 0; // client counter
    let t = c.t();
    let mut cur = c.first("Cli");
    while let Some(o) = cur {
        let next = c.next(o); // CYCLE_SAFE
        // remove old clients
        if c.vs(o, "__tSel") < t - 1.0 {
            // last selection is old?
            let b = c.shooks(o).expect("brochure");
            c.delete(b); // remove supplier brochure entry
            c.delete(o); // remove client entry
        } else {
            i += 1;
        }
        cur = next;
    }

    i as f64
}

/// Open job positions for a firm in capital-good sector.
fn jo1(c: &mut Ctx) -> f64 {
    let x = c.v("_L1d") - c.vl("_L1", 1);
    max(x, 0.0)
}

/// Labor employed by firm in capital-good sector. Includes R&D labor.
fn l1(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 8];
    let par = c.parent();

    v[1] = c.vs(par, "L1"); // total labor in sector 1
    v[2] = c.vs(par, "L1d"); // total labor demand in sector 1
    v[3] = c.vs(par, "L1rd"); // R&D labor in sector 1
    v[4] = c.vs(par, "L1dRD"); // R&D labor demand in sector 1
    v[5] = c.v("_L1d"); // firm total labor demand
    v[6] = c.v("_L1rd"); // firm R&D labor
    v[7] = c.v("_L1dRD"); // firm R&D labor demand

    // total workers in firm after possible shortage of workers
    min(v[5], v[6] + if v[2] > v[4] { (v[5] - v[7]) * (v[1] - v[3]) / (v[2] - v[4]) } else { 0.0 })
}

/// Labor demand of firm in capital-good sector. Includes R&D labor.
fn l1d(c: &mut Ctx) -> f64 {
    let x = c.v("_L1dRD");
    let y = c.v("_Q1");
    let z = c.v("_Btau");
    let par = c.parent();
    x + y / (z * c.vs(par, "m1"))
}

/// R&D labor demand of firm in capital-good sector.
fn l1d_rd(c: &mut Ctx) -> f64 {
    let x = c.v("_RD");
    let lab = labsupl2(c);
    x / c.vs(lab, "w")
}

/// R&D labor employed by firm in capital-good sector.
fn l1rd(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let v1 = c.vs(par, "L1dRD");
    if v1 > 0.0 {
        let x = c.v("_L1dRD");
        x * c.vs(par, "L1rd") / v1
    } else {
        0.0
    }
}

/// Profit (before taxes) of firm in capital-good sector.
fn pi1(c: &mut Ctx) -> f64 {
    let x = c.v("_S1") + c.v("_iD1");
    x - c.v("_W1") - c.v("_i1")
}

/// Effective output of firm in capital-good sector.
fn q1e(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 8];

    v[0] = c.v("_Q1"); // planned production
    v[1] = c.v("_L1"); // effective labor available
    v[2] = c.v("_L1d"); // desired total workers

    if v[1] >= v[2] {
        return v[0]; // produce as planned
    }

    v[3] = c.v("_L1rd"); // effective R&D workers
    v[4] = c.v("_L1dRD"); // desired R&D workers

    // adjustment factor
    v[5] = if v[2] > v[4] { 1.0 - (v[1] - v[3]) / (v[2] - v[4]) } else { 1.0 };

    // adjust all pending orders, supplying at least one machine
    let t = c.t();
    let mut cur = c.first("Cli");
    while let Some(o) = cur {
        if c.vs(o, "__tOrd") == t {
            // order in this period?
            v[6] = c.vs(o, "__nOrd") - c.vs(o, "__nCan"); // existing net orders
            v[7] = min((v[6] * v[5]).floor(), v[6] - 1.0);
            v[0] -= v[7];
            c.incrs(o, "__nCan", v[7]);
        }
        cur = c.next(o);
    }

    max(v[0], 0.0) // avoid negative in part. cases
}

/// Sales of firm in capital-good sector.
fn s1(c: &mut Ctx) -> f64 {
    let x = c.v("_p1");
    x * c.v("_Q1e")
}

/// Total wages paid by firm in capital-good sector.
fn w1(c: &mut Ctx) -> f64 {
    let x = c.v("_L1");
    let lab = labsupl2(c);
    x * c.vs(lab, "w")
}

/// Unit cost of production (1 machine) of firm in capital-good sector. Use sectoral average (pool of sector 1 workers).
fn c1(c: &mut Ctx) -> f64 {
    c.v("_Atau"); // ensure innovation process ok
    let lab = labsupl2(c);
    let w = c.vs(lab, "w");
    let b = c.v("_Btau");
    let par = c.parent();
    w / (b * c.vs(par, "m1"))
}

/// Planned (expected) unit production cost of machine when employed for production.
fn c_tau(c: &mut Ctx) -> f64 {
    let lab = labsupl2(c);
    let w = c.vs(lab, "w");
    w / c.v("_Atau")
}

/// Market share of firm in capital-good sector. Fair market share if sector didn't produce.
fn f1(c: &mut Ctx) -> f64 {
    let par = c.parent();
    let v1 = c.vs(par, "Q1e");
    if v1 > 0.0 { c.v("_Q1e") / v1 } else { 1.0 / c.vs(par, "F1") }
}

/// Interest paid by firm in capital-good sector.
fn i1(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    let x = c.vl("_Deb1", 1);
    x * c.vs(fin, "r")
}

/// Interest received from deposits by firm in capital-good sector.
fn id1(c: &mut Ctx) -> f64 {
    let fin = finsecl2(c);
    let x = c.vl("_NW1", 1);
    x * c.vs(fin, "r")
}

/// Bank credit supply available (new debt) to firm in capital-good sector. Function called multiple times in single time step.
fn cs1a(c: &mut Ctx) -> f64 {
    let x = c.v("_Deb1max") - c.v("_Deb1");
    max(x, 0.0)
}

pub fn register(r: &mut Reg) {
    r.extend([
        ("_Atau", EqDef::Fn(atau)),
        ("_Deb1max", EqDef::Fn(deb1max)),
        ("_Div1", EqDef::Fn(div1)),
        ("_NC", EqDef::Fn(nc)),
        ("_Q1", EqDef::Fn(q1)),
        ("_RD", EqDef::Fn(rd)),
        ("_Tax1", EqDef::Fn(tax1)),
        ("_p1", EqDef::Fn(p1)),
        ("_BC", EqDef::Fn(bc)),
        ("_D1", EqDef::Fn(d1)),
        ("_HC", EqDef::Fn(hc)),
        ("_JO1", EqDef::Fn(jo1)),
        ("_L1", EqDef::Fn(l1)),
        ("_L1d", EqDef::Fn(l1d)),
        ("_L1dRD", EqDef::Fn(l1d_rd)),
        ("_L1rd", EqDef::Fn(l1rd)),
        ("_Pi1", EqDef::Fn(pi1)),
        ("_Q1e", EqDef::Fn(q1e)),
        ("_S1", EqDef::Fn(s1)),
        ("_W1", EqDef::Fn(w1)),
        ("_c1", EqDef::Fn(c1)),
        ("_cTau", EqDef::Fn(c_tau)),
        ("_f1", EqDef::Fn(f1)),
        ("_i1", EqDef::Fn(i1)),
        ("_iD1", EqDef::Fn(id1)),
        ("_CS1a", EqDef::Fn(cs1a)),
        // dummy equations (the value is written by other equations)
        ("_Btau", EqDef::Fn(super::dummy)),
        ("_CD1", EqDef::Fn(super::dummy)),
        ("_CD1c", EqDef::Fn(super::dummy)),
        ("_CS1", EqDef::Fn(super::dummy)),
        ("_Deb1", EqDef::Fn(super::dummy)),
        ("_NW1", EqDef::Fn(super::dummy)),
        ("_NW1p", EqDef::Dummy("_Q1")),
        ("_imi", EqDef::Fn(super::dummy)),
        ("_inn", EqDef::Fn(super::dummy)),
    ]);
}
