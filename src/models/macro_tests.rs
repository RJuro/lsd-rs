//! Small LSD models written to exercise one family of macros each (tests/lsd-models/), ported here.
//! Every macro result is recorded with `WRITE("Rec", ..)`, so the computation trace shows each one.
use crate::model::{Ctx, EqDef};

pub fn get(name: &str) -> Option<Vec<(&'static str, EqDef)>> {
    Some(match name {
        "MacroStats" => stats::equations(),
        "MacroSort" => sortm::equations(),
        "MacroSearch" => searchm::equations(),
        "MacroSearchFast" => searchm_fast::equations(),
        "MacroObjects" => objm::equations(),
        "MacroHooks" => hooksm::equations(),
        _ => return None,
    })
}

pub const NAMES: &[&str] = &["MacroStats", "MacroSort", "MacroSearch", "MacroSearchFast", "MacroObjects", "MacroHooks"];

mod stats {
    use super::*;
    fn x(c: &mut Ctx) -> f64 {

        c.uniform_int(0.0, 6.0)
    }

    fn y(c: &mut Ctx) -> f64 {

        c.rnd() * 10.0
    }

    fn n(c: &mut Ctx) -> f64 {

        -1.0 - c.uniform_int(0.0, 4.0)
    }

    fn zs(c: &mut Ctx) -> f64 {

        c.vl("Zs", 1) + c.rnd() - 0.5
    }

    fn rstats(c: &mut Ctx) -> f64 {
        c.use_nan();
        let g1 = c.search("Group").unwrap();
        let g2 = c.next(g1).unwrap();
        let r = c.whtave("X", "Y"); c.write("Rec", r);
        let r = c.sum("X"); c.write("Rec", r);
        let r = c.count("Item"); c.write("Rec", r);
        let r = c.ave("Y"); c.write("Rec", r);
        let r = c.sums(g2, "X"); c.write("Rec", r);
        let r = c.meds(g2, "Y"); c.write("Rec", r);
        let r = c.sd_cnds(g2, "Y", "Cat", "==", 2.0); c.write("Rec", r);
        let r = c.perc("Y", 0.9); c.write("Rec", r);
        let r = c.min_cnd("X", "Cat", ">=", 2.0); c.write("Rec", r);
        for r in c.stats(g2, "X") { c.write("Rec", r); }
        c.t()
    }

    fn gstats(c: &mut Ctx) -> f64 {
        c.use_nan();
        c.use_saved();
        let w = [1.0, 2.0, 0.5, 3.0, 1.5, 2.0];
        let par = c.parent();
        let g1 = c.searchs(par, "Group").unwrap();
        let r = c.sum("X"); c.write("Rec", r);
        let r = c.suml("X", 1); c.write("Rec", r);
        let r = c.sums(g1, "X"); c.write("Rec", r);
        let r = c.sumls(g1, "X", 2); c.write("Rec", r);
        let r = c.sum_cnd("X", "Cat", "==", 2.0); c.write("Rec", r);
        let r = c.sum_cndl("X", "Y", ">", 4.5, 1); c.write("Rec", r);
        let r = c.sum_cnds(g1, "X", "X", "<=", 3.0); c.write("Rec", r);
        let r = c.sum_cndls(g1, "X", "Cat", "!=", 1.0, 2); c.write("Rec", r);
        let r = c.max("Y"); c.write("Rec", r);
        let r = c.maxl("Y", 1); c.write("Rec", r);
        let r = c.maxs(g1, "Y"); c.write("Rec", r);
        let r = c.maxls(g1, "Y", 2); c.write("Rec", r);
        let r = c.max_cnd("Y", "Cat", "!=", 1.0); c.write("Rec", r);
        let r = c.max_cndl("Y", "X", "NNAN", 0.0, 1); c.write("Rec", r);
        let r = c.max_cnds(g1, "Y", "Cat", "GT", 1.0); c.write("Rec", r);
        let r = c.max_cndls(g1, "Y", "Y", "LT", 5.0, 2); c.write("Rec", r);
        let r = c.min("X"); c.write("Rec", r);
        let r = c.minl("X", 1); c.write("Rec", r);
        let r = c.mins(g1, "X"); c.write("Rec", r);
        let r = c.minls(g1, "X", 2); c.write("Rec", r);
        let r = c.min_cnd("X", "Y", "LT", 5.0); c.write("Rec", r);
        let r = c.min_cndl("X", "X", "GE", 2.0, 1); c.write("Rec", r);
        let r = c.min_cnds(g1, "X", "Cat", "LE", 2.0); c.write("Rec", r);
        let r = c.min_cndls(g1, "X", "X", "EQ", 3.0, 2); c.write("Rec", r);
        let r = c.ave("X"); c.write("Rec", r);
        let r = c.avel("X", 1); c.write("Rec", r);
        let r = c.aves(g1, "X"); c.write("Rec", r);
        let r = c.avels(g1, "X", 2); c.write("Rec", r);
        let r = c.ave_cnd("X", "X", "EQ", 3.0); c.write("Rec", r);
        let r = c.ave_cndl("X", "X", "NE", 3.0, 1); c.write("Rec", r);
        let r = c.ave_cnds(g1, "X", "Cat", "=", 3.0); c.write("Rec", r);
        let r = c.ave_cndls(g1, "X", "Cat", "==", 99.0, 2); c.write("Rec", r);
        let r = c.whtave("X", "W"); c.write("Rec", r);
        let r = c.whtavel("X", "W", 1); c.write("Rec", r);
        let r = c.whtaves(g1, "X", "W"); c.write("Rec", r);
        let r = c.whtavels(g1, "X", "W", 2); c.write("Rec", r);
        let r = c.whtave_cnd("X", "W", "Cat", "==", 99.0); c.write("Rec", r);
        let r = c.whtave_cndl("X", "W", "Y", ">=", 0.5, 1); c.write("Rec", r);
        let r = c.whtave_cnds(g1, "X", "W", "X", "<", 4.0); c.write("Rec", r);
        let r = c.whtave_cndls(g1, "X", "W", "Cat", "==", 2.0, 2); c.write("Rec", r);
        let r = c.sd("Y"); c.write("Rec", r);
        let r = c.sdl("Y", 1); c.write("Rec", r);
        let r = c.sds(g1, "Y"); c.write("Rec", r);
        let r = c.sdls(g1, "Y", 2); c.write("Rec", r);
        let r = c.sd_cnd("Y", "Cat", "==", 2.0); c.write("Rec", r);
        let r = c.sd_cndl("Y", "Y", ">", 4.5, 1); c.write("Rec", r);
        let r = c.sd_cnds(g1, "Y", "X", "<=", 3.0); c.write("Rec", r);
        let r = c.sd_cndls(g1, "Y", "Cat", "!=", 1.0, 2); c.write("Rec", r);
        let r = c.med("X"); c.write("Rec", r);
        let r = c.medl("X", 1); c.write("Rec", r);
        let r = c.meds(g1, "X"); c.write("Rec", r);
        let r = c.medls(g1, "X", 2); c.write("Rec", r);
        let r = c.med_cnd("X", "Cat", "!=", 1.0); c.write("Rec", r);
        let r = c.med_cndl("X", "X", "NNAN", 0.0, 1); c.write("Rec", r);
        let r = c.med_cnds(g1, "X", "Cat", "GT", 1.0); c.write("Rec", r);
        let r = c.med_cndls(g1, "X", "Y", "LT", 5.0, 2); c.write("Rec", r);
        let r = c.perc("Y", 0.3); c.write("Rec", r);
        let r = c.percl("Y", 0.3, 1); c.write("Rec", r);
        let r = c.percs(g1, "Y", 0.3); c.write("Rec", r);
        let r = c.percls(g1, "Y", 0.3, 2); c.write("Rec", r);
        let r = c.perc_cnd("Y", 0.3, "Y", "LT", 5.0); c.write("Rec", r);
        let r = c.perc_cndl("Y", 0.3, "X", "GE", 2.0, 1); c.write("Rec", r);
        let r = c.perc_cnds(g1, "Y", 0.3, "Cat", "LE", 2.0); c.write("Rec", r);
        let r = c.perc_cndls(g1, "Y", 0.3, "X", "EQ", 3.0, 2); c.write("Rec", r);
        let r = c.count("Item"); c.write("Rec", r);
        let r = c.counts(g1, "Item"); c.write("Rec", r);
        let r = c.count_cnd("Item", "X", "EQ", 3.0); c.write("Rec", r);
        let r = c.count_cndl("Item", "X", "NE", 3.0, 1); c.write("Rec", r);
        let r = c.count_cnds(g1, "Item", "Cat", "=", 3.0); c.write("Rec", r);
        let r = c.count_cndls(g1, "Item", "Cat", "==", 99.0, 2); c.write("Rec", r);
        let r = c.count_all("Item"); c.write("Rec", r);
        let r = c.count_alls(g1, "Item"); c.write("Rec", r);
        let r = c.count_all_cnd("Item", "Cat", "==", 99.0); c.write("Rec", r);
        let r = c.count_all_cndl("Item", "Y", ">=", 0.5, 1); c.write("Rec", r);
        let r = c.count_all_cnds(g1, "Item", "X", "<", 4.0); c.write("Rec", r);
        let r = c.count_all_cndls(g1, "Item", "Cat", "==", 2.0, 2); c.write("Rec", r);
        for r in c.stat("X") { c.write("Rec", r); }
        for r in c.statl("Y", 1) { c.write("Rec", r); }
        for r in c.stats(g1, "X") { c.write("Rec", r); }
        for r in c.statls(g1, "Y", 2) { c.write("Rec", r); }
        for r in c.stat_cnd("X", "Cat", "GT", 1.0) { c.write("Rec", r); }
        for r in c.stat_cndl("Y", "Y", "LT", 5.0, 1) { c.write("Rec", r); }
        for r in c.stat_cnds(g1, "X", "X", "GE", 2.0) { c.write("Rec", r); }
        for r in c.stat_cndls(g1, "Y", "Cat", "LE", 2.0, 2) { c.write("Rec", r); }
        for r in c.stat("N") { c.write("Rec", r); }
        for r in c.stat_cnd("N", "Cat", "==", 2.0) { c.write("Rec", r); }
        for r in c.stat_cnd("N", "Cat", "==", 99.0) { c.write("Rec", r); }
        let r = c.mave("Zs", 4.0); c.write("Rec", r);
        let r = c.mavel("Zs", 3.0, 1); c.write("Rec", r);
        let r = c.maves(g1, "Zs", 5.0); c.write("Rec", r);
        let r = c.mavels(g1, "Zs", 2.0, 2); c.write("Rec", r);
        let r = c.mave("Zs", -2.0); c.write("Rec", r);
        let r = c.whtmave("Zs", 3.0, &w); c.write("Rec", r);
        let r = c.whtmavel("Zs", 4.0, &w, 1); c.write("Rec", r);
        let r = c.whtmaves(g1, "Zs", 3.0, &w); c.write("Rec", r);
        let r = c.whtmavels(g1, "Zs", 5.0, &w, 2); c.write("Rec", r);
        let r = c.max("N"); c.write("Rec", r);
        let r = c.min("N"); c.write("Rec", r);
        let r = c.med("N"); c.write("Rec", r);
        let r = c.sd("N"); c.write("Rec", r);
        let r = c.count_all("Group"); c.write("Rec", r);
        let r = c.count_all_cnd("Group", "Zs", ">", 0.0); c.write("Rec", r);
        c.no_search();
        let r = c.sum("X"); c.write("Rec", r);
        let r = c.count("Item"); c.write("Rec", r);
        let r = c.ave_cnd("Y", "Cat", "==", 1.0); c.write("Rec", r);
        let r = c.maxl("X", 1); c.write("Rec", r);
        c.use_search();
        c.t()
    }

    fn istats(c: &mut Ctx) -> f64 {
        c.use_nan();
        let r = c.count("Item"); c.write("Rec", r);
        let r = c.count_all("Item"); c.write("Rec", r);
        let r = c.count_all_cnd("Item", "Cat", ">", 1.0); c.write("Rec", r);
        let r = c.count_cnd("Item", "X", "<", 3.0); c.write("Rec", r);
        let r = c.sum("X"); c.write("Rec", r);
        let r = c.ave_cnd("Y", "Cat", "!=", 2.0); c.write("Rec", r);
        let r = c.max_cndl("X", "Cat", "==", 3.0, 1); c.write("Rec", r);
        for r in c.stat("X") { c.write("Rec", r); }
        for r in c.stat_cnd("Y", "Cat", "<", 3.0) { c.write("Rec", r); }
        c.t()
    }

    pub fn equations() -> Vec<(&'static str, EqDef)> {
        vec![("X", EqDef::Fn(x)), ("Y", EqDef::Fn(y)), ("N", EqDef::Fn(n)), ("Zs", EqDef::Fn(zs)), ("RStats", EqDef::Fn(rstats)),
             ("GStats", EqDef::Fn(gstats)), ("IStats", EqDef::Fn(istats))]
    }
}

mod sortm {
    use super::*;
    use crate::macros::{DOWN, UP};
    fn f(c: &mut Ctx) -> f64 {

        c.rnd()
    }

    fn k(c: &mut Ctx) -> f64 {

        c.uniform_int(0.0, 9.0)
    }

    fn k2(c: &mut Ctx) -> f64 {

        c.rnd()
    }

    fn lk(c: &mut Ctx) -> f64 {

        c.uniform_int(0.0, 3.0)
    }

    fn sorter(c: &mut Ctx) -> f64 {
        let par = c.parent();
        let g1 = c.searchs(par, "Group").unwrap();
        c.init_tsearch("Item");
        c.sort("Item", "F", UP);
        let mut cur = c.first("Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sort("Item", "K", UP);
        let mut cur = c.first("Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sort("Item", "K", DOWN);
        let mut cur = c.first("Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sortl("Item", "Lk", UP, 1);
        let mut cur = c.first("Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sort2("Item", "K", "K2", UP);
        let mut cur = c.first("Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sort2("Item", "K", "K2", DOWN);
        let mut cur = c.first("Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sort2l("Item", "Lk", "K2", UP, 1);
        let mut cur = c.first("Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sorts(g1, "Item", "K", "down");
        let mut cur = c.firsts(g1, "Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sortls(g1, "Item", "Lk", UP, 2);
        let mut cur = c.firsts(g1, "Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sort2s(g1, "Item", "K", "K2", "Down");
        let mut cur = c.firsts(g1, "Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        c.sort2ls(g1, "Item", "Lk", "K2", "up", 1);
        let mut cur = c.firsts(g1, "Item"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.write("Rec", id); cur = c.next(o); }
        let n = c.count("Item");
        let o = c.tsearch("Item", 1.0); let id = c.vs(o, "Id"); c.write("Rec", id);
        let o = c.tsearch("Item", (n / 2.0).ceil()); let id = c.vs(o, "Id"); c.write("Rec", id);
        let o = c.tsearch("Item", n); let id = c.vs(o, "Id"); c.write("Rec", id);
        let o = c.search("Item").unwrap(); let r = c.search_inst(o); c.write("Rec", r);
        let o = c.tsearch("Item", n); let r = c.search_inst(o); c.write("Rec", r);
        let o = c.tsearch("Item", 1.0); let r = c.search_insts(g1, o); c.write("Rec", r);
        c.t()
    }

    pub fn equations() -> Vec<(&'static str, EqDef)> {
        vec![("F", EqDef::Fn(f)), ("K", EqDef::Fn(k)), ("K2", EqDef::Fn(k2)), ("Lk", EqDef::Fn(lk)), ("Sorter", EqDef::Fn(sorter))]
    }
}

mod searchm {
    use super::*;
    fn p(c: &mut Ctx) -> f64 {

        c.rnd() + 0.1
    }

    fn q(c: &mut Ctx) -> f64 {

        c.rnd() + 0.1
    }

    fn v(c: &mut Ctx) -> f64 {

        c.uniform_int(1.0, 5.0)
    }

    fn search(c: &mut Ctx) -> f64 {
        let par = c.parent(); let g1 = c.searchs(par, "Pop").unwrap();
        let n = c.count("Agent");
        let x = c.init_tsearch("Agent"); c.write("Rec", x);
        let k = 1.0; let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = n; let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = (n / 2.0).ceil(); let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = c.uniform_int(1.0, n); let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = c.uniform_int(1.0, n); let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearcht("Agent", n + 5.0); c.write("Rec", x);
        let o = c.tsearch("Agent", n + 3.0); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearchs(g1, "Agent"); c.write("Rec", x);
        let m = c.counts(g1, "Agent"); let k = c.uniform_int(1.0, m); let o = c.tsearchs(g1, "Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearchts(g1, "Agent", 0.0); c.write("Rec", x);
        let o = c.tsearchs(g1, "Agent", 1.0); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearch_cnd("V"); c.write("Rec", x);
        let o = c.tsearch_cnd("V", 1.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 2.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 3.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 4.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 5.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 99.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let x = c.init_tsearch_cnds(g1, "V"); c.write("Rec", x);
        let v = c.uniform_int(1.0, 5.0); let o = c.tsearch_cnds(g1, "V", v); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnds(g1, "V", 3.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let tot = c.sum("P");
        let o = c.rnddraw_tot("Agent", "P", tot); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_tot("Agent", "P", tot); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_tot("Agent", "P", tot); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = { let t1 = c.suml("Q", 1); c.rnddraw_totl("Agent", "Q", 1, t1) }; let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = { let t1 = c.sums(g1, "P"); c.rnddraw_tots(g1, "Agent", "P", t1) }; let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = { let t1 = c.sumls(g1, "Q", 2); c.rnddraw_totls(g1, "Agent", "Q", 2, t1) }; let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw("Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawl("Agent", "Q", 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraws(g1, "Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawls(g1, "Agent", "Q", 2); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fair("Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw("Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawl("Agent", "Q", 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraws(g1, "Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawls(g1, "Agent", "Q", 2); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fair("Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent").unwrap(); let o1 = c.rnddraw_fair("Agent").unwrap();
        let x = c.search_inst(o); c.write("Rec", x);
        let x = c.search_insts(g1, o); c.write("Rec", x);
        let x = c.search_inst(o1); c.write("Rec", x);
        let x = c.search_insts(o1, o1); c.write("Rec", x);
        let x = c.search_insts(o, o1); c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent").unwrap(); let o1 = c.rnddraw_fair("Agent").unwrap();
        let x = c.search_inst(o); c.write("Rec", x);
        let x = c.search_insts(g1, o); c.write("Rec", x);
        let x = c.search_inst(o1); c.write("Rec", x);
        let x = c.search_insts(o1, o1); c.write("Rec", x);
        let x = c.search_insts(o, o1); c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent").unwrap(); let o1 = c.rnddraw_fair("Agent").unwrap();
        let x = c.search_inst(o); c.write("Rec", x);
        let x = c.search_insts(g1, o); c.write("Rec", x);
        let x = c.search_inst(o1); c.write("Rec", x);
        let x = c.search_insts(o1, o1); c.write("Rec", x);
        let x = c.search_insts(o, o1); c.write("Rec", x);
        let o = c.search_cndl("V", 3.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndls(g1, "V", 3.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndl("V", 4.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndls(g1, "V", 4.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndl("V", 5.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndls(g1, "V", 5.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        c.no_search();
        let o = c.search_cndl("V", 2.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndl("V", 99.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let x = c.count("Agent"); c.write("Rec", x);
        c.use_search();
        c.t()
    }

    pub fn equations() -> Vec<(&'static str, EqDef)> {
        vec![("P", EqDef::Fn(p)), ("Q", EqDef::Fn(q)), ("V", EqDef::Fn(v)), ("Search", EqDef::Fn(search))]
    }
}

mod searchm_fast {
    use super::*;
    fn p(c: &mut Ctx) -> f64 {

        c.rnd() + 0.1
    }

    fn q(c: &mut Ctx) -> f64 {

        c.rnd() + 0.1
    }

    fn v(c: &mut Ctx) -> f64 {

        c.uniform_int(1.0, 5.0)
    }

    fn search(c: &mut Ctx) -> f64 {
        c.use_pointer_check();
        let par = c.parent(); let g1 = c.searchs(par, "Pop").unwrap();
        let n = c.count("Agent");
        let x = c.init_tsearch("Agent"); c.write("Rec", x);
        let k = 1.0; let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = n; let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = (n / 2.0).ceil(); let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = c.uniform_int(1.0, n); let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let k = c.uniform_int(1.0, n); let o = c.tsearch("Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearcht("Agent", n + 5.0); c.write("Rec", x);
        let o = c.tsearch("Agent", n + 3.0); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearchs(g1, "Agent"); c.write("Rec", x);
        let m = c.counts(g1, "Agent"); let k = c.uniform_int(1.0, m); let o = c.tsearchs(g1, "Agent", k); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearchts(g1, "Agent", 0.0); c.write("Rec", x);
        let o = c.tsearchs(g1, "Agent", 1.0); let x = c.vs(o, "Id"); c.write("Rec", x);
        let x = c.init_tsearch_cnd("V"); c.write("Rec", x);
        let o = c.tsearch_cnd("V", 1.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 2.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 3.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 4.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 5.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnd("V", 99.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let x = c.init_tsearch_cnds(g1, "V"); c.write("Rec", x);
        let v = c.uniform_int(1.0, 5.0); let o = c.tsearch_cnds(g1, "V", v); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.tsearch_cnds(g1, "V", 3.0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let tot = c.sum("P");
        let o = c.rnddraw_tot("Agent", "P", tot); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_tot("Agent", "P", tot); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_tot("Agent", "P", tot); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = { let t1 = c.suml("Q", 1); c.rnddraw_totl("Agent", "Q", 1, t1) }; let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = { let t1 = c.sums(g1, "P"); c.rnddraw_tots(g1, "Agent", "P", t1) }; let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = { let t1 = c.sumls(g1, "Q", 2); c.rnddraw_totls(g1, "Agent", "Q", 2, t1) }; let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw("Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawl("Agent", "Q", 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraws(g1, "Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawls(g1, "Agent", "Q", 2); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fair("Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw("Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawl("Agent", "Q", 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraws(g1, "Agent", "P"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddrawls(g1, "Agent", "Q", 2); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fair("Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent"); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent").unwrap(); let o1 = c.rnddraw_fair("Agent").unwrap();
        let x = c.search_inst(o); c.write("Rec", x);
        let x = c.search_insts(g1, o); c.write("Rec", x);
        let x = c.search_inst(o1); c.write("Rec", x);
        let x = c.search_insts(o1, o1); c.write("Rec", x);
        let x = c.search_insts(o, o1); c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent").unwrap(); let o1 = c.rnddraw_fair("Agent").unwrap();
        let x = c.search_inst(o); c.write("Rec", x);
        let x = c.search_insts(g1, o); c.write("Rec", x);
        let x = c.search_inst(o1); c.write("Rec", x);
        let x = c.search_insts(o1, o1); c.write("Rec", x);
        let x = c.search_insts(o, o1); c.write("Rec", x);
        let o = c.rnddraw_fairs(g1, "Agent").unwrap(); let o1 = c.rnddraw_fair("Agent").unwrap();
        let x = c.search_inst(o); c.write("Rec", x);
        let x = c.search_insts(g1, o); c.write("Rec", x);
        let x = c.search_inst(o1); c.write("Rec", x);
        let x = c.search_insts(o1, o1); c.write("Rec", x);
        let x = c.search_insts(o, o1); c.write("Rec", x);
        let o = c.search_cndl("V", 3.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndls(g1, "V", 3.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndl("V", 4.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndls(g1, "V", 4.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndl("V", 5.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndls(g1, "V", 5.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        c.no_search();
        let o = c.search_cndl("V", 2.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.search_cndl("V", 99.0, 1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let x = c.count("Agent"); c.write("Rec", x);
        c.use_search();
        c.t()
    }

    pub fn equations() -> Vec<(&'static str, EqDef)> {
        vec![("P", EqDef::Fn(p)), ("Q", EqDef::Fn(q)), ("V", EqDef::Fn(v)), ("Search", EqDef::Fn(search))]
    }
}

mod objm {
    use super::*;
    fn a(c: &mut Ctx) -> f64 {

        c.vl("A", 1) + c.rnd()
    }

    fn b(c: &mut Ctx) -> f64 {

        let a = c.v("A"); a * 2.0 + c.rnd()
    }

    fn w(c: &mut Ctx) -> f64 {

        let l = c.vl("W", 1); l * 0.5 + c.rnd()
    }

    fn z(c: &mut Ctx) -> f64 {

        let w = c.v("W"); w * 2.0 + c.rnd()
    }

    fn dyn_(c: &mut Ctx) -> f64 {
        let tt = c.m.t;
        c.use_nan(); c.default_result(0.5);
        let par = c.parent(); let f1 = c.searchs(par, "Firm").unwrap();
        c.v("A");
        let x = c.last_calc("A"); c.write("Rec", x);
        let x = c.recalc("A"); c.write("Rec", x);
        let x = c.last_calc("A"); c.write("Rec", x);
        let x = c.v("A"); c.write("Rec", x);
        let x = c.vl("A", 1); c.write("Rec", x);
        let x = c.vl("A", 2); c.write("Rec", x);
        let x = c.last_calcs(f1, "B"); c.write("Rec", x);
        let x = c.recalcs(f1, "B"); c.write("Rec", x);
        c.update();
        let x = c.last_calc("B"); c.write("Rec", x);
        let x = c.recalcs(f1, "B"); c.write("Rec", x);
        let x = c.vs(f1, "B"); c.write("Rec", x);
        c.update_rec();
        let x = c.sum("W"); c.write("Rec", x);
        let w = c.search("Worker").unwrap();
        let x = c.last_calcs(w, "Z"); c.write("Rec", x);
        let x = c.recalcs(w, "W"); c.write("Rec", x);
        let x = c.vs(w, "W"); c.write("Rec", x);
        let x = c.vls(w, "W", 1); c.write("Rec", x);
        let x = c.vls(w, "W", 2); c.write("Rec", x);
        let x = c.vs(w, "Z"); c.write("Rec", x);
        c.updates(w);
        let x = c.last_calcs(w, "W"); c.write("Rec", x);
        c.update_recs(f1);
        let ex = c.search("Worker").unwrap();
        let w = c.addnobj("Worker", 2).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobjl("Worker", 1, tt - 1).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobjl("Worker", 1, 0).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addobjl("Worker", tt - 1); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobj_ex("Worker", 1, ex).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobj_exl("Worker", 2, ex, tt).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addobj_exl("Worker", ex, tt); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobjs(f1, "Worker", 1).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobjls(f1, "Worker", 1, tt - 1).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobj_exs(f1, "Worker", 1, ex).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addnobj_exls(f1, "Worker", 1, ex, tt).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addobjls(f1, "Worker", tt - 1); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        let w = c.addobj_exls(f1, "Worker", ex, tt); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        assert!(c.addnobj("Worker", 0).is_none());
        let x = c.count("Worker"); c.write("Rec", x);
        let x = { let tt = c.m.t as f64; c.sum_cnd("Z", "Born", "==", tt) }; c.write("Rec", x);
        let mut cur = c.first("Worker"); while let Some(o) = cur { let nx = c.next(o); if (tt as f64) - c.vs(o, "Born") > 3.0 { c.delete(o); } cur = nx; }
        let x = c.count("Worker"); c.write("Rec", x);
        c.no_search();
        let x = c.v("A"); c.write("Rec", x);
        let x = c.last_calc("B"); c.write("Rec", x);
        let x = c.sum("W"); c.write("Rec", x);
        let x = c.count("Worker"); c.write("Rec", x);
        c.use_search();
        if c.v("Zero") == 1.0 && (tt as f64 % 5.0) == 0.0 {
        c.use_zero_instance(); let mut cur = c.first("Worker"); while let Some(o) = cur { let nx = c.next(o); c.delete(o); cur = nx; }
        let x = c.count("Worker"); c.write("Rec", x);
        let x = c.sum("W"); c.write("Rec", x);
        let x = c.max("W"); c.write("Rec", x);
        let x = c.ave_cnd("W", "Born", ">", 0.0); c.write("Rec", x);
        let x = c.search("Worker").is_none() as i32 as f64; c.write("Rec", x);
        let x = c.rnddraw_fair("Worker").is_none() as i32 as f64; c.write("Rec", x);
        let x = c.count_all("Worker"); c.write("Rec", x);
        let w = c.addnobj("Worker", 3).unwrap(); let mut cur = Some(w); while let Some(o) = cur { c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id); c.writes(o, "Born", tt as f64); cur = c.next(o); } let x = c.vs(w, "W"); c.write("Rec", x); let x = c.last_calcs(w, "W"); c.write("Rec", x);
        c.no_zero_instance(); }
        let x = c.count("Worker"); c.write("Rec", x);
        if tt == 12 && c.v("Zero") == 2.0 { c.abort(); }
        c.t()
    }

    pub fn equations() -> Vec<(&'static str, EqDef)> {
        vec![("A", EqDef::Fn(a)), ("B", EqDef::Fn(b)), ("W", EqDef::Fn(w)), ("Z", EqDef::Fn(z)), ("Dyn", EqDef::Fn(dyn_))]
    }
}

mod hooksm {
    use super::*;

    struct WExt { skill: f64, hist: Vec<f64> }
    struct FExt { total: f64, n: i32 }

    fn hist(c: &mut Ctx) -> f64 {

        c.rnd()
    }

    fn stat(c: &mut Ctx) -> f64 {
        let p = c.p;
        let h = c.v("Hist");
        c.ext_mut::<WExt>(p).hist.push(h);
        let e = c.ext_mut::<WExt>(p); e.skill = e.skill * 0.9 + 0.1 * h;
        let x = c.ext::<WExt>(p).skill; c.write("Rec", x);
        let x = c.ext::<WExt>(p).hist.len() as f64; c.write("Rec", x);
        let x = { let o = c.parent(); c.vs(o, "Level") }; c.write("Rec", x);
        let x = { let o = c.grandparent(); c.vs(o, "Top") }; c.write("Rec", x);
        let x = { let o = c.root(); c.vs(o, "Top") }; c.write("Rec", x);
        let x = (c.name() == "Worker") as i32 as f64; c.write("Rec", x);
        let x = c.next_brother().is_none() as i32 as f64; c.write("Rec", x);
        let x = { let o = c.shook().unwrap(); c.vs(o, "Level") }; c.write("Rec", x);
        let x = c.last_t(); c.write("Rec", x);
        if c.count_hook() == 0 { c.addhook(1); }
        let nx = c.next_brother(); c.write_hook(0, nx);
        let o = c.hook(0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        c.ext::<WExt>(p).skill
    }

    fn boss(c: &mut Ctx) -> f64 {
        let tt = c.m.t;
        let par = c.parent(); let f1 = c.searchs(par, "Firm").unwrap();
        if tt == 1 {
        c.addhook(3);
        let mut cur = c.first("Worker"); while let Some(o) = cur { let id = c.vs(o, "Id"); c.add_ext(o, WExt { skill: id * 0.01, hist: Vec::new() }); let p = c.p; c.write_shooks(o, p); cur = c.next(o); }
        let p = c.p; c.add_ext(p, FExt { total: 7.0, n: 0 });
        }
        let o = c.addobj("Worker"); c.incr("NextId", 1.0); let id = c.v("NextId"); c.writes(o, "Id", id);
        let id = c.vs(o, "Id"); c.add_ext(o, WExt { skill: id * 0.01, hist: Vec::new() }); let p = c.p; c.write_shooks(o, p);
        let newest = o;
        if c.count("Worker") > 5.0 { let o = c.search("Worker").unwrap(); c.delete(o); }
        let first = c.search("Worker"); c.write_hook(0, first); c.write_hook(1, newest);
        let h0 = c.hook(0).unwrap(); let nx = c.next(h0); c.write_hook(2, nx);
        let o = c.hook(0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.hook(1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.hook(2); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let x = c.count_hook() as f64; c.write("Rec", x);
        c.addhooks(f1, 4); let p = c.p; c.write_hooks(f1, 3, p);
        let x = c.count_hooks(f1) as f64; c.write("Rec", x);
        let o = c.hooks(f1, 0); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let x = { let h = c.hooks(f1, 3).unwrap(); c.vs(h, "Level") }; c.write("Rec", x);
        c.write_shook(newest);
        let o = c.shook(); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        let o = c.shooks(f1); let x = match o { Some(o) => c.vs(o, "Id"), None => -1.0 }; c.write("Rec", x);
        c.write_shooks(f1, None); c.write_shook(None);
        let x = c.shook().is_none() as i32 as f64; c.write("Rec", x);
        if tt == 5 { c.addhook(6); let x = c.count_hook() as f64; c.write("Rec", x); c.addhook(2); let x = c.count_hook() as f64; c.write("Rec", x); c.addhook(3); let x = c.count_hook() as f64; c.write("Rec", x); c.write_hook(0, None); }
        let x = c.hook(0).is_none() as i32 as f64; c.write("Rec", x);
        let first = c.search("Worker"); c.write_hook(0, first);
        let x = { let o = c.parent(); c.vs(o, "Top") }; c.write("Rec", x);
        let x = { let o = c.up(newest); c.vs(o, "Level") }; c.write("Rec", x);
        let x = { let o = c.grandparents(newest); c.vs(o, "Top") }; c.write("Rec", x);
        let x = c.next(newest).is_none() as i32 as f64; c.write("Rec", x);
        let x = c.names(newest).as_bytes()[0] as f64; c.write("Rec", x);
        let x = c.name().len() as f64; c.write("Rec", x);
        let x = { let o = c.root(); c.vs(o, "Top") }; c.write("Rec", x);
        let s = c.sum("Stat"); let p = c.p; let e = c.ext_mut::<FExt>(p); e.total += s; e.n += 1;
        let x = { let p = c.p; c.ext::<FExt>(p).total }; c.write("Rec", x);
        let x = { let p = c.p; c.ext::<FExt>(p).n as f64 }; c.write("Rec", x);
        let x = c.ext::<FExt>(f1).total; c.write("Rec", x);
        let x = c.ext::<WExt>(newest).skill; c.write("Rec", x);
        let x = c.ext::<WExt>(newest).hist.len() as f64; c.write("Rec", x);
        c.add_ext(newest, WExt { skill: 5.5, hist: Vec::new() }); c.ext_mut::<WExt>(newest).skill = 6.5;
        let x = c.ext::<WExt>(newest).skill; c.write("Rec", x);
        c.delete_ext(newest); c.add_ext(newest, WExt { skill: 1.5, hist: Vec::new() });
        if tt == 3 { let p = c.p; c.delete_ext(p); c.add_ext(p, FExt { total: 100.0, n: 0 }); }
        c.t()
    }

    pub fn equations() -> Vec<(&'static str, EqDef)> {
        vec![("Hist", EqDef::Fn(hist)), ("Stat", EqDef::Fn(stat)), ("Boss", EqDef::Fn(boss))]
    }
}
