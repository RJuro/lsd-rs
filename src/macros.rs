//! The rest of LSD's equation macros as `Ctx` methods: statistics, sorting, searches, hooks, extensions.
//!
//! Naming scheme (the macro name in snake_case, then suffixes in LSD's order):
//!   `sum(lab)`                        SUM(X)           over the calling object `c.p`
//!   `suml(lab, lag)`                  SUML(X, L)       lagged
//!   `sums(o, lab)`, `sumls(o, lab, lag)`   SUMS, SUMLS  from object `o` instead of `c.p`
//!   `sum_cnd(lab, lab2, op, value)`   SUM_CND(X,T,R,V) only instances where `lab2 op value`
//!   `sum_cndl`, `sum_cnds`, `sum_cndls`  the same with lag / object / both (lag last, object first)
//! Every statistics family (SUM MAX MIN AVE WHTAVE SD MED PERC COUNT COUNT_ALL STAT) has these 8 forms;
//! `op` is LSD's operator string ("==", "!=", ">", ">=", "<", "<=", "EQ", "NE", "GT", "GE", "LT", "LE", "NNAN").
//! PERC takes its percentile after the label (`perc(lab, 0.9)`), WHTAVE its weight label (`whtave(lab, wlab)`).
//! STAT returns LSD's 7 values `[n, mean, variance, max, min, median, sd]` (the `v[]` array STAT fills).
//! MAVE/WHTMAVE: `mave(lab, per)`, `mavel(lab, per, lag)`, `maves(o, ..)`, `mavels(o, ..)`; `whtmave*` add `&[f64]` weights after `per`.
//! Other macros keep their name with `l` (lag) before `s` (object): `addnobjls`, `rnddraw_totls`, `init_tsearchts` ...
//! `UP`/`DOWN` are the SORT directions. Extensions are generic: `add_ext::<T>(o, value)`, `ext::<T>(o)`, `ext_mut::<T>(o)`.
//!
//! Differences from C++ worth knowing:
//!  * Hooks and `search`-like results are `Option<ObjId>` (C++ NULL = None).
//!  * Searches honour `NO_SEARCH` as LSD does (own element, then the direct sons' own elements for statistics).
//!  * LSD's turbo-search tree is a snapshot of the instance chain; here it is a flat vector with the same contents.
//!  * `SORT` reproduces the macOS libc `qsort` (FreeBSD quicksort with heapsort fallback) because LSD's comparator never
//!    returns 0, so the order of ties depends on the algorithm. On Linux (glibc) ties would come out differently.
//!  * `SEARCH_INST` depends on the header the C++ model includes (`use_pointer_check()` for `fun_head_fast.h`).
//!  * `STAT`'s maximum starts from DBL_MIN, `COUNT_ALL`/`STAT` walk from LSD's odd starting points: reproduced as is.
//!  * State the macros need (turbo trees, DEFAULT_RESULT) lives in `Model::ext` under a reserved key.

use std::any::Any;
use std::collections::HashMap;

use crate::model::{Ctx, Model, ObjId};

macro_rules! fail {
    ($($a:tt)*) => { panic!("LSD error: {}", format!($($a)*)) };
}

pub const UP: &str = "UP";
pub const DOWN: &str = "DOWN";

/// Optional condition `lab2 op value`.
type Cnd<'a> = Option<(&'a str, &'a str, f64)>;

// ---------------------------------------------------------------------------------------- model state

const AUX: ObjId = usize::MAX; // reserved key in Model::ext

#[derive(Default)]
struct Aux {
    def_res: f64,
    ptr_check: bool,
    /// INIT_TSEARCH: (parent, object label) -> (instances by position, declared total)
    turbo: HashMap<(ObjId, String), (Vec<ObjId>, i64)>,
    /// INIT_TSEARCH_CND: (parent, object label) -> (variable label, value -> first instance with it).
    /// (LSD 8.1 aborts with an error when an instance of such a bridge is deleted: `delete_obj` computes the
    /// search variable of the already emptied object. Deleting here simply leaves a stale entry.)
    turbo_cnd: HashMap<(ObjId, String), (String, HashMap<u64, ObjId>)>,
}

fn aux(m: &mut Model) -> &mut Aux {
    m.ext.entry(AUX).or_insert_with(|| Box::new(Aux::default())).downcast_mut::<Aux>().unwrap()
}

/// DEFAULT_RESULT value (LSD only uses it as the result of an equation that sets none, which cannot happen here).
pub fn default_result(m: &mut Model) -> f64 {
    aux(m).def_res
}

fn bits(v: f64) -> u64 {
    if v == 0.0 { 0 } else { v.to_bits() } // unordered_map<double>: +0 and -0 are one key
}

// ---------------------------------------------------------------------------------------- helpers

/// LSD object::search_var(caller = o, lab, no_search = NO_SEARCH, search_sons = true): the element, found from `o`.
fn search_var(m: &Model, o: ObjId, caller: Option<ObjId>, lab: &str, no_up: bool, no_search: bool, sons: bool) -> Option<(ObjId, usize)> {
    let obj = m.o(o);
    if let Some(&i) = obj.index.get(lab) {
        return Some((o, i));
    }
    if no_search && !sons {
        return None;
    }
    for b in &obj.sons {
        if let Some(h) = b.head {
            if caller.map_or(true, |c| *m.o(h).label != *m.o(c).label) {
                if let Some(r) = search_var(m, h, Some(o), lab, true, no_search, false) {
                    return Some(r);
                }
            }
        }
    }
    if no_search || no_up {
        return None;
    }
    match obj.up {
        Some(u) if Some(u) != caller => search_var(m, u, Some(o), lab, false, false, false),
        _ => None,
    }
}

/// search_var_err(this = o, lab, NO_SEARCH, .., search_sons = true). None only for LSD's "zero instance" case.
fn sv(m: &Model, o: ObjId, lab: &str) -> Option<(ObjId, usize)> {
    match search_var(m, o, Some(o), lab, false, m.no_search, true) {
        Some(r) => Some(r),
        None if !m.no_zero_instance => None,
        None => fail!("element '{}' is missing (from object '{}') for a statistic", lab, m.label(o)),
    }
}

/// First instance of `o`'s brotherhood (`o->up->search(o->label)`).
fn first_brother(m: &Model, o: ObjId) -> ObjId {
    match m.o(o).up {
        None => o,
        Some(u) => m.o(u).sons.iter().find(|b| *b.label == *m.o(o).label).and_then(|b| b.head).unwrap_or(o),
    }
}

/// object::search_err(lab, NO_SEARCH): self, direct son, descendants, then up the tree.
fn search_err(m: &Model, o: ObjId, lab: &str) -> Option<ObjId> {
    if m.no_search {
        let obj = m.o(o);
        let r = if *obj.label == *lab { Some(o) } else { obj.sons.iter().find(|b| *b.label == *lab).and_then(|b| b.head) };
        if r.is_none() && m.no_zero_instance {
            fail!("object '{}' not directly under '{}' (NO_SEARCH enabled!)", lab, m.label(o));
        }
        r
    } else {
        m.cycle_first(o, lab)
    }
}

fn opcode(op: &str) -> i32 {
    match op {
        "==" | "=" | "EQ" => 0,
        "!=" | "=!" | "NE" => 1,
        ">" | "GT" => 2,
        ">=" | "=>" | "GE" => 3,
        "<" | "LT" => 4,
        "<=" | "=<" | "LE" => 5,
        "NNAN" => 6,
        _ => fail!("cannot compare with '{}': invalid logical relational operator", op),
    }
}

fn check_cond(v: f64, lopc: i32, w: f64) -> bool {
    match lopc {
        0 => v == w,
        1 => v != w,
        2 => v > w,
        3 => v >= w,
        4 => v < w,
        5 => v <= w,
        6 => !v.is_nan(),
        _ => false,
    }
}

/// object::search_inst(obj, &pos, &checked) with checked = -1: walk `this` and its `hyper_next` followers; unless
/// pointer checking is on, also search below each. `pos` is the index within the level where `obj` turned up.
fn inst_rec(m: &Model, this: ObjId, obj: ObjId, no_ptr_chk: bool, pos: &mut f64) {
    let mut found = false;
    let mut i = 1.0;
    let mut cur = Some(this);
    while let Some(c) = cur {
        if *pos != 0.0 {
            return;
        }
        if c == obj {
            *pos = i;
            return;
        }
        if !found {
            if no_ptr_chk || m.o(c).label != m.o(obj).label {
                for b in &m.o(c).sons {
                    if *pos != 0.0 {
                        break;
                    }
                    if let Some(h) = b.head {
                        inst_rec(m, h, obj, no_ptr_chk, pos);
                    }
                }
            } else {
                found = true;
            }
        }
        cur = m.hyper_next(c);
        i += 1.0;
    }
}

// ---------------------------------------------------------------------------------------- qsort

/// macOS libc `qsort` (Apple Libc 1500+, FreeBSD's Bentley-McIlroy quicksort with a depth limit that falls back to
/// heapsort), element by element, so the comparator sees the same pairs in the same order and ties land where the
/// C library puts them. Checked against the machine's own `qsort` in the unit test below.
pub fn bsd_qsort<T: Copy>(a: &mut [T], cmp: &mut dyn FnMut(T, T) -> i32) {
    let depth = 2 * (usize::BITS - 1 - a.len().max(1).leading_zeros()) as i32; // DEPTH(n)
    qsort_rec(a, cmp, depth);
}

/// Insertion sort; with a swap limit it gives up (false) once it has made more swaps than allowed.
fn isort<T: Copy>(a: &mut [T], cmp: &mut dyn FnMut(T, T) -> i32, swap_limit: usize) -> bool {
    let mut swaps = 0;
    for pm in 1..a.len() {
        let mut pl = pm;
        while pl > 0 && cmp(a[pl - 1], a[pl]) > 0 {
            a.swap(pl, pl - 1);
            swaps += 1;
            if swap_limit > 0 && swaps > swap_limit {
                return false;
            }
            pl -= 1;
        }
    }
    true
}

fn med3<T: Copy>(a: &[T], x: usize, y: usize, z: usize, cmp: &mut dyn FnMut(T, T) -> i32) -> usize {
    if cmp(a[x], a[y]) < 0 {
        if cmp(a[y], a[z]) < 0 { y } else if cmp(a[x], a[z]) < 0 { z } else { x }
    } else if cmp(a[y], a[z]) > 0 { y } else if cmp(a[x], a[z]) < 0 { x } else { z }
}

fn qsort_rec<T: Copy>(mut a: &mut [T], cmp: &mut dyn FnMut(T, T) -> i32, mut depth: i32) {
    loop {
        let n = a.len();
        let out_of_depth = depth <= 0;
        depth -= 1;
        if out_of_depth {
            return heapsort(a, cmp);
        }
        if n <= 7 {
            isort(a, cmp, 0);
            return;
        }
        let (mut pl, mut pm, mut pn) = (0, n / 2, n - 1);
        if n > 40 {
            let d = n / 8;
            pl = med3(a, pl, pl + d, pl + 2 * d, cmp);
            pm = med3(a, pm - d, pm, pm + d, cmp);
            pn = med3(a, pn - 2 * d, pn - d, pn, cmp);
        }
        pm = med3(a, pl, pm, pn, cmp);
        a.swap(0, pm);
        let (mut pa, mut pb, mut pc, mut pd) = (1, 1, n - 1, n - 1);
        let mut swapped = false;
        loop {
            while pb <= pc {
                let r = cmp(a[pb], a[0]);
                if r > 0 { break }
                if r == 0 {
                    swapped = true;
                    a.swap(pa, pb);
                    pa += 1;
                }
                pb += 1;
            }
            while pb <= pc {
                let r = cmp(a[pc], a[0]);
                if r < 0 { break }
                if r == 0 {
                    swapped = true;
                    a.swap(pc, pd);
                    pd -= 1;
                }
                pc -= 1;
            }
            if pb > pc { break }
            a.swap(pb, pc);
            swapped = true;
            pb += 1;
            pc -= 1;
        }
        let d1 = pa.min(pb - pa);
        for i in 0..d1 { a.swap(i, pb - d1 + i) }
        let d1 = (pd - pc).min(n - pd - 1);
        for i in 0..d1 { a.swap(pb + i, n - d1 + i) }
        if !swapped && isort(a, cmp, 1 + n / 4) {
            return; // nearly sorted already
        }
        let (d1, d2) = (pb - pa, pd - pc);
        if d1 <= d2 {
            if d1 > 1 {
                qsort_rec(&mut a[..d1], cmp, depth);
            }
            if d2 > 1 {
                a = &mut a[n - d2..];
            } else {
                return;
            }
        } else {
            if d2 > 1 {
                qsort_rec(&mut a[n - d2..], cmp, depth);
            }
            if d1 > 1 {
                a = &mut a[..d1];
            } else {
                return;
            }
        }
    }
}

/// BSD heapsort (Knuth vol. 3 p. 145), the depth-limit fallback; items are numbered from 1.
fn heapsort<T: Copy>(a: &mut [T], cmp: &mut dyn FnMut(T, T) -> i32) {
    let mut nmemb = a.len();
    if nmemb <= 1 {
        return;
    }
    let mut l = nmemb / 2 + 1;
    loop {
        l -= 1;
        if l == 0 { break }
        let mut par = l;
        while par * 2 <= nmemb {
            let mut child = par * 2;
            if child < nmemb && cmp(a[child - 1], a[child]) < 0 { child += 1 }
            if cmp(a[child - 1], a[par - 1]) <= 0 { break }
            a.swap(par - 1, child - 1);
            par = child;
        }
    }
    while nmemb > 1 {
        let k = a[nmemb - 1];
        a[nmemb - 1] = a[0];
        nmemb -= 1;
        let mut par = 1;
        while par * 2 <= nmemb {
            let mut child = par * 2;
            if child < nmemb && cmp(a[child - 1], a[child]) < 0 { child += 1 }
            a[par - 1] = a[child - 1];
            par = child;
        }
        loop {
            let child = par;
            par = child / 2;
            if child == 1 || cmp(k, a[par - 1]) < 0 {
                a[child - 1] = k;
                break;
            }
            a[child - 1] = a[par - 1];
        }
    }
}

// ---------------------------------------------------------------------------------------- the macros

/// Generates the 8 forms of a statistic (see the top of the file) from `core(o, lag, args.., cnd)`.
macro_rules! family {
    ($core:ident, $ret:ty, [$($a:ident: $t:ty),*],
     $f:ident, $fl:ident, $fs:ident, $fls:ident, $fc:ident, $fcl:ident, $fcs:ident, $fcls:ident) => {
        pub fn $f(&mut self, $($a: $t),*) -> $ret { let p = self.p; self.$core(p, 0, $($a,)* None) }
        pub fn $fl(&mut self, $($a: $t,)* lag: i32) -> $ret { let p = self.p; self.$core(p, lag, $($a,)* None) }
        pub fn $fs(&mut self, o: ObjId, $($a: $t),*) -> $ret { self.$core(o, 0, $($a,)* None) }
        pub fn $fls(&mut self, o: ObjId, $($a: $t,)* lag: i32) -> $ret { self.$core(o, lag, $($a,)* None) }
        pub fn $fc(&mut self, $($a: $t,)* lab2: &str, op: &str, value: f64) -> $ret {
            let p = self.p;
            self.$core(p, 0, $($a,)* Some((lab2, op, value)))
        }
        pub fn $fcl(&mut self, $($a: $t,)* lab2: &str, op: &str, value: f64, lag: i32) -> $ret {
            let p = self.p;
            self.$core(p, lag, $($a,)* Some((lab2, op, value)))
        }
        pub fn $fcs(&mut self, o: ObjId, $($a: $t,)* lab2: &str, op: &str, value: f64) -> $ret {
            self.$core(o, 0, $($a,)* Some((lab2, op, value)))
        }
        pub fn $fcls(&mut self, o: ObjId, $($a: $t,)* lab2: &str, op: &str, value: f64, lag: i32) -> $ret {
            self.$core(o, lag, $($a,)* Some((lab2, op, value)))
        }
    };
}

impl<'a> Ctx<'a> {
    // ---- the loop every statistic shares

    /// LSD validates the elements, then returns the object holding the (last) element and the operator code.
    fn lookup(&self, o: ObjId, labs: &[&str], cnd: &Cnd) -> Option<(ObjId, i32)> {
        let mut up = None;
        for l in labs {
            up = Some(sv(self.m, o, l)?.0);
        }
        let lopc = match cnd {
            Some((lab2, op, _)) => {
                let c = opcode(op);
                sv(self.m, o, lab2)?;
                c
            }
            None => -1,
        };
        Some((up.unwrap(), lopc))
    }

    /// Visit `start` and its followers (`next`, or `hyper_next` for COUNT_ALL) that satisfy the condition and are not
    /// being deleted. Per instance: successor first (instances may delete themselves), condition, then `f`.
    fn each(&mut self, o: ObjId, start: Option<ObjId>, lag: i32, cnd: &Cnd, lopc: i32, hyper: bool, f: &mut dyn FnMut(&mut Self, ObjId)) {
        let mut cur = start;
        while let Some(c) = cur {
            let nx = if hyper { self.m.hyper_next(c) } else { self.m.o(c).next };
            let ok = match cnd {
                None => true,
                Some((lab2, _, value)) => check_cond(self.m.cal(c, Some(o), lab2, lag), lopc, *value),
            };
            if ok && self.m.exists(c) && !self.m.o(c).deleting {
                f(self, c);
            }
            cur = nx;
        }
    }

    fn sum_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        let Some((up, lopc)) = self.lookup(o, &[lab], &cnd) else { return 0.0 };
        let start = first_brother(self.m, up);
        let mut tot = 0.0;
        self.each(o, Some(start), lag, &cnd, lopc, false, &mut |s, c| tot += s.m.cal(c, Some(o), lab, lag));
        tot
    }

    fn max_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        let Some((up, lopc)) = self.lookup(o, &[lab], &cnd) else { return f64::NAN };
        let start = first_brother(self.m, up);
        let (mut tot, mut n) = (-f64::MAX, 0);
        self.each(o, Some(start), lag, &cnd, lopc, false, &mut |s, c| {
            let temp = s.m.cal(c, Some(o), lab, lag);
            if tot < temp { tot = temp }
            n += 1;
        });
        if n > 0 { tot } else { f64::NAN }
    }

    fn min_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        let Some((up, lopc)) = self.lookup(o, &[lab], &cnd) else { return f64::NAN };
        let start = first_brother(self.m, up);
        let (mut tot, mut n) = (f64::MAX, 0);
        self.each(o, Some(start), lag, &cnd, lopc, false, &mut |s, c| {
            let temp = s.m.cal(c, Some(o), lab, lag);
            if tot > temp { tot = temp }
            n += 1;
        });
        if n > 0 { tot } else { f64::NAN }
    }

    fn ave_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        let Some((up, lopc)) = self.lookup(o, &[lab], &cnd) else { return f64::NAN };
        let start = first_brother(self.m, up);
        let (mut tot, mut n) = (0.0, 0);
        self.each(o, Some(start), lag, &cnd, lopc, false, &mut |s, c| {
            tot += s.m.cal(c, Some(o), lab, lag);
            n += 1;
        });
        if n > 0 { tot / n as f64 } else { f64::NAN }
    }

    /// WHTAVE: sum of lab * wlab (LSD does not divide). The group is that of the *second* element.
    fn whtave_core(&mut self, o: ObjId, lag: i32, lab: &str, wlab: &str, cnd: Cnd) -> f64 {
        let Some((up, lopc)) = self.lookup(o, &[lab, wlab], &cnd) else { return 0.0 };
        let start = first_brother(self.m, up);
        let mut tot = 0.0;
        self.each(o, Some(start), lag, &cnd, lopc, false, &mut |s, c| {
            let x = s.m.cal(c, Some(o), lab, lag);
            let w = s.m.cal(c, Some(o), wlab, lag);
            tot += x * w;
        });
        tot
    }

    fn perc_core(&mut self, o: ObjId, lag: i32, lab: &str, pc: f64, cnd: Cnd) -> f64 {
        if !(0.0..=1.0).contains(&pc) {
            fail!("percentile '{}' is invalid", pc);
        }
        let Some((up, lopc)) = self.lookup(o, &[lab], &cnd) else { return f64::NAN };
        let start = first_brother(self.m, up);
        let mut vals: Vec<f64> = Vec::new();
        self.each(o, Some(start), lag, &cnd, lopc, false, &mut |s, c| vals.push(s.m.cal(c, Some(o), lab, lag)));
        let n = vals.len();
        if n == 0 {
            return f64::NAN;
        }
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let x = pc * (n - 1) as f64 + 1.0; // C=1 variant a la NumPy
        let fx = x.floor() as usize;
        let vx = vals[fx - 1];
        let vx1 = if fx < n { vals[fx] } else { vx };
        vx + x.fract() * (vx1 - vx)
    }

    fn med_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        self.perc_core(o, lag, lab, 0.5, cnd)
    }

    fn sd_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        let Some((up, lopc)) = self.lookup(o, &[lab], &cnd) else { return f64::NAN };
        let start = first_brother(self.m, up);
        let (mut tot, mut tot2, mut n) = (0.0, 0.0, 0);
        self.each(o, Some(start), lag, &cnd, lopc, false, &mut |s, c| {
            let x = s.m.cal(c, Some(o), lab, lag);
            tot += x;
            tot2 += x * x;
            n += 1;
        });
        if n > 0 {
            let nn = n as f64;
            let m = tot / nn;
            let var = tot2 / nn - m * m; // pow(m, 2)
            if var > 0.0 { var.sqrt() } else { 0.0 } // max(var, 0)
        } else {
            f64::NAN
        }
    }

    fn count_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        let Some(cur) = search_err(self.m, o, lab) else { return 0.0 };
        let lopc = match &cnd {
            Some((lab2, op, _)) => {
                let c = opcode(op);
                if sv(self.m, o, lab2).is_none() { return f64::NAN }
                c
            }
            None => -1,
        };
        let mut n = 0;
        self.each(o, Some(cur), lag, &cnd, lopc, false, &mut |_, _| n += 1);
        n as f64
    }

    /// COUNT_ALL: all instances of `lab` in the model below the first bridge of the parent, across branches.
    fn count_all_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> f64 {
        let up = self.m.o(o).up.expect("COUNT_ALL needs a parent");
        let cur = match self.m.o(up).sons.first().and_then(|b| b.head) {
            Some(h) => search_err(self.m, h, lab),
            None => search_err(self.m, o, lab),
        };
        let Some(cur) = cur else { return 0.0 };
        let lopc = match &cnd {
            Some((lab2, op, _)) => {
                let c = opcode(op);
                if sv(self.m, o, lab2).is_none() { return f64::NAN }
                c
            }
            None => -1,
        };
        let mut n = 0;
        self.each(o, Some(cur), lag, &cnd, lopc, true, &mut |_, _| n += 1);
        n as f64
    }

    /// STAT: LSD walks from the object that holds the element (not from the first brother) and each
    /// instance computes the value for itself as caller.
    fn stat_core(&mut self, o: ObjId, lag: i32, lab: &str, cnd: Cnd) -> [f64; 7] {
        let none = [0.0, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN];
        let Some((up, lopc)) = self.lookup(o, &[lab], &cnd) else { return none };
        let mut r = [0.0, 0.0, 0.0, f64::MIN_POSITIVE, f64::MAX, 0.0, 0.0]; // DBL_MIN, not -DBL_MAX: LSD's max is wrong for all-negative data
        let mut vals: Vec<f64> = Vec::new();
        self.each(o, Some(up), lag, &cnd, lopc, false, &mut |s, c| {
            let val = s.m.cal(c, Some(c), lab, lag);
            r[1] += val;
            r[2] += val * val;
            if val > r[3] { r[3] = val }
            if val < r[4] { r[4] = val }
            vals.push(val);
        });
        let n = vals.len();
        r[0] = n as f64;
        if n > 0 {
            let nn = n as f64;
            r[1] /= nn;
            r[2] = r[2] / nn - r[1] * r[1];
            r[6] = if r[2] >= 0.0 { r[2].sqrt() } else { f64::NAN };
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            r[5] = if n % 2 == 0 { (vals[n / 2 - 1] + vals[n / 2]) / 2.0 } else { vals[n / 2] };
        } else {
            r = none;
        }
        r
    }

    family!(sum_core, f64, [lab: &str], sum, suml, sums, sumls, sum_cnd, sum_cndl, sum_cnds, sum_cndls);
    family!(max_core, f64, [lab: &str], max, maxl, maxs, maxls, max_cnd, max_cndl, max_cnds, max_cndls);
    family!(min_core, f64, [lab: &str], min, minl, mins, minls, min_cnd, min_cndl, min_cnds, min_cndls);
    family!(ave_core, f64, [lab: &str], ave, avel, aves, avels, ave_cnd, ave_cndl, ave_cnds, ave_cndls);
    family!(whtave_core, f64, [lab: &str, wlab: &str], whtave, whtavel, whtaves, whtavels, whtave_cnd, whtave_cndl, whtave_cnds, whtave_cndls);
    family!(sd_core, f64, [lab: &str], sd, sdl, sds, sdls, sd_cnd, sd_cndl, sd_cnds, sd_cndls);
    family!(med_core, f64, [lab: &str], med, medl, meds, medls, med_cnd, med_cndl, med_cnds, med_cndls);
    family!(perc_core, f64, [lab: &str, pc: f64], perc, percl, percs, percls, perc_cnd, perc_cndl, perc_cnds, perc_cndls);
    family!(count_core, f64, [lab: &str], count, countl, counts, countls, count_cnd, count_cndl, count_cnds, count_cndls);
    family!(count_all_core, f64, [lab: &str], count_all, count_alll, count_alls, count_allls, count_all_cnd, count_all_cndl, count_all_cnds, count_all_cndls);
    family!(stat_core, [f64; 7], [lab: &str], stat, statl, stats, statls, stat_cnd, stat_cndl, stat_cnds, stat_cndls);

    // ---- moving averages over time

    fn mav_core(&mut self, o: ObjId, caller: ObjId, lab: &str, per: f64, w: Option<&[f64]>, lag: i32) -> f64 {
        if (!self.m.use_nan && per.is_nan()) || per.is_infinite() || per.abs() < 1.0 {
            fail!("period '{}' is invalid for moving average '{}'", per, lab);
        }
        let Some(cv) = sv(self.m, o, lab) else { return f64::NAN };
        let (per, maxlag) = if per < 0.0 { (-per, self.m.var(cv).num_lag as i32) } else { (per, 0) };
        let (mut sumv, mut sumw, mut i) = (0.0, 0.0, 0);
        while (i as f64) < per {
            if i + lag - self.m.t >= maxlag {
                break;
            }
            match w {
                None => sumv += self.m.cal_var(cv, Some(caller), i + lag),
                Some(w) => {
                    sumv += self.m.cal_var(cv, Some(caller), i + lag) * w[i as usize];
                    sumw += w[i as usize];
                }
            }
            i += 1;
        }
        if w.is_none() { sumv / i as f64 } else { sumv / sumw }
    }
    pub fn mave(&mut self, lab: &str, per: f64) -> f64 { let p = self.p; self.mav_core(p, p, lab, per, None, 0) }
    pub fn mavel(&mut self, lab: &str, per: f64, lag: i32) -> f64 { let p = self.p; self.mav_core(p, p, lab, per, None, lag) }
    pub fn maves(&mut self, o: ObjId, lab: &str, per: f64) -> f64 { self.mav_core(o, o, lab, per, None, 0) }
    pub fn mavels(&mut self, o: ObjId, lab: &str, per: f64, lag: i32) -> f64 { self.mav_core(o, o, lab, per, None, lag) }
    pub fn whtmave(&mut self, lab: &str, per: f64, w: &[f64]) -> f64 { let p = self.p; self.mav_core(p, p, lab, per, Some(w), 0) }
    pub fn whtmavel(&mut self, lab: &str, per: f64, w: &[f64], lag: i32) -> f64 { let p = self.p; self.mav_core(p, p, lab, per, Some(w), lag) }
    pub fn whtmaves(&mut self, o: ObjId, lab: &str, per: f64, w: &[f64]) -> f64 { self.mav_core(o, o, lab, per, Some(w), 0) }
    pub fn whtmavels(&mut self, o: ObjId, lab: &str, per: f64, w: &[f64], lag: i32) -> f64 { self.mav_core(o, o, lab, per, Some(w), lag) }

    // ---- sorting

    /// lsdqsort: reorder the instances of `obj` by `var1` (then `var2`); returns the new first.
    /// The bridge is the one holding `var1` (SORT) or the direct son `obj` of `o` (SORT2).
    fn sort_core(&mut self, o: ObjId, obj: &str, var1: &str, var2: Option<&str>, dir: &str, lag: i32) -> ObjId {
        let (cvo, _) = sv(self.m, o, var1).unwrap_or_else(|| fail!("element '{}' is missing for sorting", var1));
        if *self.m.o(cvo).label != *obj {
            fail!("element '{}' is missing (object '{}') for sorting", var1, obj);
        }
        let holder = self.m.o(cvo).up.unwrap_or_else(|| fail!("object '{}' is missing for sorting", obj));
        let parent = if var2.is_some() { o } else { holder };
        let bi = self.m.o(parent).sons.iter().position(|b| *b.label == *obj).unwrap_or_else(|| fail!("object '{}' is missing for sorting", obj));
        let head = self.m.o(parent).sons[bi].head.unwrap_or_else(|| fail!("all instances of object '{}' were deleted", obj));
        let mut list = self.m.brothers(Some(head));
        let d: String = dir.chars().take(5).collect::<String>().to_uppercase(); // strcpyn(.., 6) + strupr
        let up = match d.as_str() {
            "UP" => true,
            "DOWN" => false,
            _ => fail!("direction '{}' is invalid for sorting", dir),
        };
        {
            let m = &mut *self.m;
            let mut cmp = |x: ObjId, y: ObjId| -> i32 {
                // C++ leaves the order of the two cal() calls unspecified; GCC evaluates the left one first
                let a = m.cal(x, Some(x), var1, lag);
                let b = m.cal(y, Some(y), var1, lag);
                let (lt, gt) = if up { (a < b, a > b) } else { (a > b, a < b) };
                match var2 {
                    None => if lt { -1 } else { 1 }, // never 0: ties depend on the sort algorithm
                    Some(v2) => {
                        if lt { -1 } else if gt { 1 } else {
                            let s = m.cal(x, Some(x), v2, lag);
                            let t = m.cal(y, Some(y), v2, lag);
                            if (up && s < t) || (!up && s > t) { -1 } else { 1 }
                        }
                    }
                }
            };
            bsd_qsort(&mut list, &mut cmp);
        }
        let m = &mut *self.m;
        m.objs[parent].as_mut().unwrap().sons[bi].head = Some(list[0]);
        m.objs[parent].as_ref().unwrap().sons[bi].counter_updated.set(false);
        for w in list.windows(2) {
            m.objs[w[0]].as_mut().unwrap().next = Some(w[1]);
        }
        m.objs[*list.last().unwrap()].as_mut().unwrap().next = None;
        list[0]
    }
    /// SORT(obj, var, dir) and its lag / object forms. `dir` is `UP` or `DOWN` (any case).
    pub fn sort(&mut self, obj: &str, var: &str, dir: &str) -> ObjId { let p = self.p; self.sort_core(p, obj, var, None, dir, 0) }
    pub fn sortl(&mut self, obj: &str, var: &str, dir: &str, lag: i32) -> ObjId { let p = self.p; self.sort_core(p, obj, var, None, dir, lag) }
    pub fn sorts(&mut self, o: ObjId, obj: &str, var: &str, dir: &str) -> ObjId { self.sort_core(o, obj, var, None, dir, 0) }
    pub fn sortls(&mut self, o: ObjId, obj: &str, var: &str, dir: &str, lag: i32) -> ObjId { self.sort_core(o, obj, var, None, dir, lag) }
    /// SORT2(obj, var1, var2, dir): ties in `var1` are ordered by `var2`.
    pub fn sort2(&mut self, obj: &str, v1: &str, v2: &str, dir: &str) -> ObjId { let p = self.p; self.sort_core(p, obj, v1, Some(v2), dir, 0) }
    pub fn sort2l(&mut self, obj: &str, v1: &str, v2: &str, dir: &str, lag: i32) -> ObjId { let p = self.p; self.sort_core(p, obj, v1, Some(v2), dir, lag) }
    pub fn sort2s(&mut self, o: ObjId, obj: &str, v1: &str, v2: &str, dir: &str) -> ObjId { self.sort_core(o, obj, v1, Some(v2), dir, 0) }
    pub fn sort2ls(&mut self, o: ObjId, obj: &str, v1: &str, v2: &str, dir: &str, lag: i32) -> ObjId { self.sort_core(o, obj, v1, Some(v2), dir, lag) }

    // ---- searches and random draws

    /// INIT_TSEARCH / INIT_TSEARCHT: index the instances of son `lab` by position (`tot` = their number, 0 = count them).
    pub fn init_tsearch(&mut self, lab: &str) -> f64 { let p = self.p; self.initturbo(p, lab, 0.0) }
    pub fn init_tsearcht(&mut self, lab: &str, tot: f64) -> f64 { let p = self.p; self.initturbo(p, lab, tot) }
    pub fn init_tsearchs(&mut self, o: ObjId, lab: &str) -> f64 { self.initturbo(o, lab, 0.0) }
    pub fn init_tsearchts(&mut self, o: ObjId, lab: &str, tot: f64) -> f64 { self.initturbo(o, lab, tot) }
    /// TSEARCH(lab, n): the n-th (1-based) instance as it stood at INIT_TSEARCH (a snapshot, as in LSD).
    pub fn tsearch(&mut self, lab: &str, num: f64) -> ObjId { let p = self.p; self.turbosearch(p, lab, num) }
    pub fn tsearchs(&mut self, o: ObjId, lab: &str, num: f64) -> ObjId { self.turbosearch(o, lab, num) }

    fn initturbo(&mut self, o: ObjId, lab: &str, tot: f64) -> f64 {
        let Some(b) = self.m.o(o).sons.iter().find(|b| *b.label == *lab) else { fail!("object '{}' is missing for turbo search", lab) };
        let Some(head) = b.head else { fail!("failure when initializing object '{}' for turbo search", lab) };
        let chain = self.m.brothers(Some(head));
        let mut ltot = tot.floor() as i64;
        if ltot <= 0 {
            ltot = chain.len() as i64;
        }
        let level = if ltot > 1 { (((ltot - 1) as f64).log10() + 1e-6) as u32 + 1 } else { 1 };
        // LSD fills 10^level leaves in chain order and repeats the last instance when the chain runs out
        let leaves = (0..10usize.pow(level)).map(|i| chain[i.min(chain.len() - 1)]).collect();
        aux(self.m).turbo.insert((o, lab.to_string()), (leaves, ltot));
        ltot as f64
    }

    fn turbosearch(&mut self, o: ObjId, lab: &str, num: f64) -> ObjId {
        let Some((leaves, tot)) = aux(self.m).turbo.get(&(o, lab.to_string())) else { fail!("object '{}' is not initialized for turbo search", lab) };
        let pos = num.floor() as i64 - 1;
        if pos < 0 || pos >= *tot {
            fail!("position '{}' is invalid for turbo searching object '{}'", num, lab);
        }
        leaves[pos as usize]
    }

    /// INIT_TSEARCH_CND(lab): map value of `lab` -> first instance having it; returns the number of distinct values.
    pub fn init_tsearch_cnd(&mut self, lab: &str) -> f64 { let p = self.p; self.initturbo_cond(p, lab) }
    pub fn init_tsearch_cnds(&mut self, o: ObjId, lab: &str) -> f64 { self.initturbo_cond(o, lab) }
    /// TSEARCH_CND(lab, value): the instance with that value of `lab` (None if no instance has it).
    pub fn tsearch_cnd(&mut self, lab: &str, value: f64) -> Option<ObjId> { let p = self.p; self.turbosearch_cond(p, lab, value) }
    pub fn tsearch_cnds(&mut self, o: ObjId, lab: &str, value: f64) -> Option<ObjId> { self.turbosearch_cond(o, lab, value) }

    /// The bridge (parent, label) holding the element `lab` as seen from `o`.
    fn turbo_bridge(&self, o: ObjId, lab: &str) -> Option<(ObjId, String)> {
        let (cvo, _) = sv(self.m, o, lab)?;
        match self.m.o(cvo).up {
            Some(u) => Some((u, self.m.o(cvo).label.to_string())),
            None => fail!("element '{}' is at root level (always single-instanced)", lab),
        }
    }

    fn initturbo_cond(&mut self, o: ObjId, lab: &str) -> f64 {
        let Some(key) = self.turbo_bridge(o, lab) else { return 0.0 };
        let b = self.m.o(key.0).sons.iter().find(|b| *b.label == *key.1).unwrap();
        let mut map = HashMap::new();
        let mut cur = b.head;
        while let Some(c) = cur {
            let nx = self.m.o(c).next;
            let v = self.m.cal(c, Some(c), lab, 0);
            map.entry(bits(v)).or_insert(c); // insert() keeps the first
            cur = nx;
        }
        let n = map.len();
        aux(self.m).turbo_cnd.insert(key, (lab.to_string(), map));
        n as f64
    }

    fn turbosearch_cond(&mut self, o: ObjId, lab: &str, value: f64) -> Option<ObjId> {
        let key = self.turbo_bridge(o, lab)?;
        match aux(self.m).turbo_cnd.get(&key) {
            Some((l, map)) if !map.is_empty() && l == lab => map.get(&bits(value)).copied(),
            _ => fail!("element '{}' is not initialized for turbo conditional search", lab),
        }
    }

    /// SEARCH_INST(obj) / SEARCH_INSTS(o, obj): the number of `obj` counting from the first instance of the type of `p`
    /// (resp. `o`) in LSD's order of instances (`hyper_next`, across parents), 0 if `obj` does not exist.
    /// This is what a model including `fun_head.h` gets (LSD's pointer check is off). For `fun_head_fast.h`
    /// (pointer check on) call `use_pointer_check()` first: the count then starts at the first instance in `obj`'s own parent.
    pub fn search_inst(&self, obj: ObjId) -> f64 { self.search_insts(self.p, obj) }
    pub fn search_insts(&self, this: ObjId, obj: ObjId) -> f64 {
        let check = self.m.ext.get(&AUX).and_then(|a| a.downcast_ref::<Aux>()).map_or(false, |a| a.ptr_check);
        let mut cur = if check && obj != this {
            if !self.m.exists(obj) {
                return 0.0;
            }
            obj
        } else {
            this
        };
        cur = first_brother(self.m, cur);
        let mut pos = 0.0;
        inst_rec(self.m, cur, obj, !check, &mut pos);
        pos
    }
    /// USE_POINTER_CHECK / NO_POINTER_CHECK: LSD's pointer checking, on in models using `fun_head_fast.h`.
    pub fn use_pointer_check(&mut self) { aux(self.m).ptr_check = true }
    pub fn no_pointer_check(&mut self) { aux(self.m).ptr_check = false }

    /// SEARCH_CNDL / SEARCH_CNDLS: like SEARCH_CND with a lag; with NO_SEARCH only the own bridge is scanned.
    pub fn search_cndl(&mut self, lab: &str, value: f64, lag: i32) -> Option<ObjId> { let p = self.p; self.search_cndls(p, lab, value, lag) }
    pub fn search_cndls(&mut self, o: ObjId, lab: &str, value: f64, lag: i32) -> Option<ObjId> {
        let (mut cur, _) = sv(self.m, o, lab)?;
        loop {
            let nx = if self.m.no_search { self.m.o(cur).next } else { self.m.hyper_next(cur) };
            if self.m.cal(cur, Some(cur), lab, lag) == value {
                return Some(cur);
            }
            cur = nx?;
        }
    }

    /// RNDDRAWL(lo, lv, lag) and RNDDRAW_TOT(lo, lv, tot): an instance with probability proportional to `lv`;
    /// the _TOT forms skip the pass that adds up the probabilities (`tot` is their sum).
    pub fn rnddrawl(&mut self, lo: &str, lv: &str, lag: i32) -> Option<ObjId> { let p = self.p; self.draw_core(p, lo, lv, lag, None) }
    pub fn rnddrawls(&mut self, o: ObjId, lo: &str, lv: &str, lag: i32) -> Option<ObjId> { self.draw_core(o, lo, lv, lag, None) }
    pub fn rnddraw_tot(&mut self, lo: &str, lv: &str, tot: f64) -> Option<ObjId> { let p = self.p; self.draw_core(p, lo, lv, 0, Some(tot)) }
    pub fn rnddraw_totl(&mut self, lo: &str, lv: &str, lag: i32, tot: f64) -> Option<ObjId> { let p = self.p; self.draw_core(p, lo, lv, lag, Some(tot)) }
    pub fn rnddraw_tots(&mut self, o: ObjId, lo: &str, lv: &str, tot: f64) -> Option<ObjId> { self.draw_core(o, lo, lv, 0, Some(tot)) }
    pub fn rnddraw_totls(&mut self, o: ObjId, lo: &str, lv: &str, lag: i32, tot: f64) -> Option<ObjId> { self.draw_core(o, lo, lv, lag, Some(tot)) }

    fn draw_core(&mut self, o: ObjId, _lo: &str, lv: &str, lag: i32, tot: Option<f64>) -> Option<ObjId> {
        if let Some(t) = tot {
            if t <= 0.0 {
                fail!("element '{}' has invalid value '{}' for random drawing", lv, t);
            }
        }
        let (first, _) = sv(self.m, o, lv)?;
        let (mut cur, mut cur1);
        match tot {
            None => {
                let mut a = 0.0;
                let mut c = Some(first);
                while let Some(x) = c {
                    let nx = self.m.o(x).next;
                    a += self.m.cal(x, Some(x), lv, lag);
                    c = nx;
                }
                if a.is_nan() || a.is_infinite() {
                    fail!("element '{}' has invalid value '{}' for random drawing", lv, a);
                }
                if a == 0.0 {
                    fail!("element '{}' has only zero values for random drawing", lv);
                }
                let b = loop {
                    let b = self.m.rng.ran1() * a;
                    if b != a { break b }
                };
                let mut a = self.m.cal(first, Some(first), lv, lag);
                cur = first;
                cur1 = self.m.o(first).next;
                while a <= b {
                    let Some(x) = cur1 else { break };
                    let nx = self.m.o(x).next;
                    a += self.m.cal(x, Some(x), lv, lag);
                    cur = x;
                    cur1 = nx;
                }
            }
            Some(tot) => {
                let b = self.m.rng.ran1() * tot;
                let mut cnext = self.m.o(first).next;
                let mut a = self.m.cal(first, Some(first), lv, lag);
                cur = first;
                cur1 = cnext;
                while a <= b {
                    let Some(x) = cur1 else { break };
                    cnext = self.m.o(x).next;
                    a += self.m.cal(x, Some(x), lv, lag);
                    cur = x;
                    cur1 = cnext;
                }
                if a > tot {
                    fail!("element '{}' has invalid value '{}' for random drawing", lv, tot);
                }
            }
        }
        Some(cur)
    }

    /// RNDDRAW_FAIRS(o, lab): an instance of `lab` with equal probabilities.
    pub fn rnddraw_fairs(&mut self, o: ObjId, lab: &str) -> Option<ObjId> {
        let first = search_err(self.m, o, lab)?;
        let n = self.m.brothers(Some(first)).len() as f64;
        let b = loop {
            let b = self.m.rng.ran1() * n;
            if b != n { break b }
        };
        let (mut a, mut cur) = (1.0, first);
        let mut cur1 = self.m.o(first).next;
        while a <= b {
            let Some(x) = cur1 else { break };
            a += 1.0;
            cur = x;
            cur1 = self.m.o(x).next;
        }
        Some(cur)
    }

    // ---- adding objects (n copies; `l` = time of last update of the new objects)

    fn add_n(&mut self, o: ObjId, lab: &str, n: usize, ex: Option<ObjId>, t_update: i32) -> Option<ObjId> {
        if n == 0 {
            return None; // LSD returns NULL; the engine's add_objects needs n >= 1
        }
        Some(self.m.add_objects(o, lab, n, ex, t_update))
    }
    pub fn addobjl(&mut self, lab: &str, tu: i32) -> ObjId { let p = self.p; self.m.add_objects(p, lab, 1, None, tu) }
    pub fn addobjls(&mut self, o: ObjId, lab: &str, tu: i32) -> ObjId { self.m.add_objects(o, lab, 1, None, tu) }
    pub fn addobj_exl(&mut self, lab: &str, ex: ObjId, tu: i32) -> ObjId { let p = self.p; self.m.add_objects(p, lab, 1, Some(ex), tu) }
    pub fn addobj_exls(&mut self, o: ObjId, lab: &str, ex: ObjId, tu: i32) -> ObjId { self.m.add_objects(o, lab, 1, Some(ex), tu) }
    pub fn addnobj(&mut self, lab: &str, n: usize) -> Option<ObjId> { let p = self.p; self.add_n(p, lab, n, None, -1) }
    pub fn addnobjl(&mut self, lab: &str, n: usize, tu: i32) -> Option<ObjId> { let p = self.p; self.add_n(p, lab, n, None, tu) }
    pub fn addnobjs(&mut self, o: ObjId, lab: &str, n: usize) -> Option<ObjId> { self.add_n(o, lab, n, None, -1) }
    pub fn addnobjls(&mut self, o: ObjId, lab: &str, n: usize, tu: i32) -> Option<ObjId> { self.add_n(o, lab, n, None, tu) }
    pub fn addnobj_ex(&mut self, lab: &str, n: usize, ex: ObjId) -> Option<ObjId> { let p = self.p; self.add_n(p, lab, n, Some(ex), -1) }
    pub fn addnobj_exl(&mut self, lab: &str, n: usize, ex: ObjId, tu: i32) -> Option<ObjId> { let p = self.p; self.add_n(p, lab, n, Some(ex), tu) }
    pub fn addnobj_exs(&mut self, o: ObjId, lab: &str, n: usize, ex: ObjId) -> Option<ObjId> { self.add_n(o, lab, n, Some(ex), -1) }
    pub fn addnobj_exls(&mut self, o: ObjId, lab: &str, n: usize, ex: ObjId, tu: i32) -> Option<ObjId> { self.add_n(o, lab, n, Some(ex), tu) }

    // ---- recomputation, updating, switches

    /// LAST_CALC / LAST_CALCS: the time the variable was last computed.
    pub fn last_calc(&mut self, lab: &str) -> f64 { let p = self.p; self.last_calcs(p, lab) }
    pub fn last_calcs(&mut self, o: ObjId, lab: &str) -> f64 {
        let r = self.m.find_var(o, lab);
        self.m.var(r).last_update as f64
    }
    /// RECALC / RECALCS: mark the variable as not computed in this step so the next request recomputes it
    /// (its lags are shifted back as if this step had not happened). Returns the value it had.
    pub fn recalc(&mut self, lab: &str) -> f64 { let p = self.p; self.recalcs(p, lab) }
    pub fn recalcs(&mut self, o: ObjId, lab: &str) -> f64 {
        let r = self.m.find_var(o, lab);
        let t = self.m.t;
        let v = &mut self.m.objs[r.0].as_mut().unwrap().vars[r.1];
        if v.last_update < t {
            return v.val[0];
        }
        let app = v.val[0];
        let nl = v.num_lag;
        for i in 0..nl {
            v.val[i] = v.val[i + 1];
        }
        v.val[nl] = if v.save && (nl as i32) + 1 <= t - v.start { v.data[(t - nl as i32 - 1 - v.start) as usize] } else { f64::NAN };
        v.last_update = t - 1;
        v.next_update = t;
        app
    }

    /// UPDATE / UPDATES(o) (non-recursive) and UPDATE_REC / UPDATE_RECS(o): compute every variable of the object
    /// (and its descendants) not yet computed in this step, with the engine's own update.
    pub fn update(&mut self) { let p = self.p; self.m.update_obj(p, false) }
    pub fn updates(&mut self, o: ObjId) { self.m.update_obj(o, false) }
    pub fn update_rec(&mut self) { let p = self.p; self.m.update_obj(p, true) }
    pub fn update_recs(&mut self, o: ObjId) { self.m.update_obj(o, true) }

    pub fn no_search(&mut self) { self.m.no_search = true }
    pub fn use_search(&mut self) { self.m.no_search = false }
    pub fn no_zero_instance(&mut self) { self.m.no_zero_instance = true }
    pub fn use_zero_instance(&mut self) { self.m.no_zero_instance = false }
    pub fn no_nan(&mut self) { self.m.use_nan = false }
    /// DEFAULT_RESULT: recorded only; LSD uses it for an equation that sets no result, which Rust equations cannot do.
    pub fn default_result(&mut self, x: f64) { aux(self.m).def_res = x }
    /// ABORT: the simulation stops at the end of the current step.
    pub fn abort(&mut self) { self.m.abort = true }

    // ---- hooks (C++ object pointers kept in an object; None = NULL)

    fn hook_slot(&self, o: ObjId, i: usize) -> usize {
        if i >= self.m.o(o).hooks.len() {
            fail!("hook {} is not available for object '{}' ({} hooks)", i, self.m.label(o), self.m.o(o).hooks.len());
        }
        i
    }
    pub fn hook(&self, i: usize) -> Option<ObjId> { self.hooks(self.p, i) }
    pub fn hooks(&self, o: ObjId, i: usize) -> Option<ObjId> { let i = self.hook_slot(o, i); self.m.o(o).hooks[i] }
    pub fn shook(&self) -> Option<ObjId> { self.m.o(self.p).hook }
    pub fn shooks(&self, o: ObjId) -> Option<ObjId> { self.m.o(o).hook }
    pub fn write_hook(&mut self, i: usize, x: impl Into<Option<ObjId>>) { let p = self.p; self.write_hooks(p, i, x) }
    pub fn write_hooks(&mut self, o: ObjId, i: usize, x: impl Into<Option<ObjId>>) {
        let i = self.hook_slot(o, i);
        self.m.objs[o].as_mut().unwrap().hooks[i] = x.into();
    }
    pub fn write_shook(&mut self, x: impl Into<Option<ObjId>>) { let p = self.p; self.write_shooks(p, x) }
    pub fn write_shooks(&mut self, o: ObjId, x: impl Into<Option<ObjId>>) { self.m.objs[o].as_mut().unwrap().hook = x.into() }
    /// ADDHOOK(n): resize the hook list to `n` entries (new ones empty).
    pub fn addhook(&mut self, n: usize) { let p = self.p; self.addhooks(p, n) }
    pub fn addhooks(&mut self, o: ObjId, n: usize) { self.m.objs[o].as_mut().unwrap().hooks.resize(n, None) }
    pub fn count_hook(&self) -> usize { self.m.o(self.p).hooks.len() }
    pub fn count_hooks(&self, o: ObjId) -> usize { self.m.o(o).hooks.len() }

    // ---- extensions: a model's own Rust struct attached to an object (C++ ADDEXT/EXT/...)

    /// ADDEXT / ADDEXT_INIT: attach `value` to `o` (replacing, and dropping, a previous extension).
    pub fn add_ext<T: Any>(&mut self, o: ObjId, value: T) { self.m.ext.insert(o, Box::new(value)); }
    /// DELETE_EXT
    pub fn delete_ext(&mut self, o: ObjId) { self.m.ext.remove(&o); }
    pub fn has_ext(&self, o: ObjId) -> bool { self.m.ext.contains_key(&o) }
    /// EXT / P_EXT / V_EXT: read the extension (a field: `c.ext::<Firm>(o).size`).
    pub fn ext<T: Any>(&self, o: ObjId) -> &T {
        self.m.ext.get(&o).and_then(|b| b.downcast_ref::<T>())
            .unwrap_or_else(|| fail!("object '{}' has no extension of type {}", self.m.label(o), std::any::type_name::<T>()))
    }
    /// WRITE_EXT / EXEC_EXT: mutable access (`c.ext_mut::<Firm>(o).size = x`).
    pub fn ext_mut<T: Any>(&mut self, o: ObjId) -> &mut T {
        let lab = self.m.label(o).to_string();
        self.m.ext.get_mut(&o).and_then(|b| b.downcast_mut::<T>())
            .unwrap_or_else(|| fail!("object '{}' has no extension of type {}", lab, std::any::type_name::<T>()))
    }

    // ---- navigation

    pub fn root(&self) -> ObjId { self.m.root }
    pub fn name(&self) -> &str { self.m.label(self.p) }
    pub fn names(&self, o: ObjId) -> &str { self.m.label(o) }
    pub fn grandparent(&self) -> ObjId { self.up(self.parent()) }
    pub fn grandparents(&self, o: ObjId) -> ObjId { self.up(self.up(o)) }
    /// NEXT (the calling object's next brother)
    pub fn next_brother(&self) -> Option<ObjId> { self.m.next(self.p) }
    /// LAST_T
    pub fn last_t(&self) -> f64 { self.m.max_step as f64 }
}

// ---------------------------------------------------------------------------------------- additions for K+S-original

impl<'a> Ctx<'a> {
    /// WRITELL( lab, x, time, lag ) / WRITELLS( o, lab, x, time, lag ): write `x` as if computed `lag` periods before
    /// `time`, the time of the update (added for K+S-original; `writel` is the `lag = 0` case).
    pub fn writell(&mut self, lab: &str, x: f64, time: i32, lag: i32) -> f64 { let p = self.p; self.m.write(p, lab, x, time, lag) }
    pub fn writells(&mut self, o: ObjId, lab: &str, x: f64, time: i32, lag: i32) -> f64 { self.m.write(o, lab, x, time, lag) }
}

#[cfg(test)]
mod tests {
    use super::bsd_qsort;
    use std::ffi::c_void;

    extern "C" {
        fn qsort(base: *mut c_void, n: usize, size: usize, cmp: extern "C" fn(*const c_void, *const c_void) -> i32);
    }
    // LSD's comparator: never 0. Elements are key << 16 | id.
    thread_local!(static LOG: std::cell::RefCell<Vec<(u64, u64)>> = Default::default());
    extern "C" fn up(a: *const c_void, b: *const c_void) -> i32 {
        let (a, b) = unsafe { (*(a as *const u64), *(b as *const u64)) };
        LOG.with(|l| l.borrow_mut().push((a, b)));
        if (a >> 16) < (b >> 16) { -1 } else { 1 }
    }

    /// The libc of the machine and `bsd_qsort` order ties identically (macOS only: Linux libc differs).
    #[test]
    #[cfg(target_os = "macos")]
    fn qsort_matches_libc() {
        let mut seed = 12345u64;
        let mut rnd = move |m: u64| { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (seed >> 33) % m };
        for n in (1..=130).chain([200, 301, 1000]) {
            for keys in [2, 5, 50, 100000] {
                for _ in 0..20 {
                    let v: Vec<u64> = (0..n).map(|i| (rnd(keys) << 16) | i as u64).collect();
                    let mut c = v.clone();
                    LOG.with(|l| l.borrow_mut().clear());
                    unsafe { qsort(c.as_mut_ptr() as *mut c_void, c.len(), 8, up) };
                    let log_c = LOG.with(|l| std::mem::take(&mut *l.borrow_mut()));
                    let mut r = v.clone();
                    let mut log_r = Vec::new();
                    bsd_qsort(&mut r, &mut |a, b| { log_r.push((a, b)); if (a >> 16) < (b >> 16) { -1 } else { 1 } });
                    assert_eq!(c, r, "n={} keys={}", n, keys);
                    assert_eq!(log_c, log_r, "comparison order, n={} keys={}", n, keys);
                }
            }
        }
    }
}
