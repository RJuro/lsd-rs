//! The LSD object tree and its evaluation rules, in compat mode: the same order of computation,
//! the same lag rule, the same object creation and deletion as LSD 8.1 (object.cpp, variab.cpp).
//! Objects live in an arena and are linked like LSD's (`next` within a bridge), so cycles,
//! deletions and additions during a step behave exactly as in LSD.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::config::{Config, Kind, ObjSpec};
use crate::rng::LsdRng;

pub type ObjId = usize;

/// A fast, non-cryptographic hasher for the label maps (the FxHash recipe); std's SipHash dominated the profile.
#[derive(Default, Clone, Copy)]
pub struct Fx(u64);
impl Hasher for Fx {
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.add(u64::from_le_bytes(c.try_into().unwrap()));
        }
        let mut tail = [0u8; 8];
        let r = chunks.remainder();
        tail[..r.len()].copy_from_slice(r);
        self.add(u64::from_le_bytes(tail) ^ r.len() as u64);
    }
    fn write_u8(&mut self, b: u8) { self.add(b as u64) }
    fn write_usize(&mut self, n: usize) { self.add(n as u64) }
    fn finish(&self) -> u64 { self.0 }
}
impl Fx {
    #[inline]
    fn add(&mut self, w: u64) { self.0 = (self.0.rotate_left(5) ^ w).wrapping_mul(0x517cc1b727220a95) }
}
pub type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;

/// The shape of an object type: its elements, son types in bridge order, parent type.
struct TypeInfo {
    index: Rc<FxMap<Rc<str>, usize>>,
    sons: Vec<u32>,
    up: Option<u32>,
}

/// Where a lookup from a given object type lands, worked out once from the type structure:
/// move up or down (first instance of a son bridge) step by step, then take element `var`.
#[derive(Clone)]
struct Plan {
    label: Rc<str>,
    steps: Vec<Step>,
    var: usize,
}

#[derive(Clone, Copy)]
enum Step {
    Up,
    Down(usize),
}
pub type Eq = fn(&mut Ctx) -> f64;

/// An equation: a function, a dummy computed by another equation (EQUATION_DUMMY),
/// or LSD's FUNCTION macro (returns the stored value when the system updates it).
#[derive(Clone, Copy, Debug)]
pub enum EqDef {
    Fn(Eq),
    Dummy(&'static str),
    Function(Eq),
    /// Registered as "close_sim": called once after the run, like LSD's close_sim()
    Hook(fn(&mut Model)),
}

#[derive(Clone, Debug)]
pub struct Var {
    pub label: Rc<str>,
    pub kind: Kind,
    pub num_lag: usize,
    pub val: Vec<f64>, // val[0] most recent, len num_lag + 1
    pub last_update: i32,
    pub next_update: i32,
    pub upd: [i32; 4], // delay, delay_range, period, period_range
    pub save: bool,
    pub data: Vec<f64>, // saved series, data[i] is time start + i
    pub start: i32,
    pub end: i32,
    pub under_computation: bool,
    pub dummy: bool,
    pub eq: Option<EqDef>, // resolved once at load, copied with the object
}

#[derive(Clone, Debug)]
pub struct Bridge {
    pub label: Rc<str>,
    pub ty: u32,
    pub counter_updated: Cell<bool>, // LSD: instance positions cached this step
    pub head: Option<ObjId>,
}

#[derive(Clone, Debug)]
pub struct Object {
    pub label: Rc<str>,
    pub ty: u32, // interned object type, compared instead of the label
    pub up: Option<ObjId>,
    pub next: Option<ObjId>,
    pub vars: Vec<Var>,
    pub index: Rc<FxMap<Rc<str>, usize>>, // var label -> position, shared by a type's instances
    pub sons: Vec<Bridge>,
    pub to_compute: bool,
    pub in_update: bool, // LSD's del_flag registration
    pub deleting: bool,
    pub hook: Option<ObjId>,         // SHOOK
    pub acounter: Cell<i32>,         // LSD's cached instance position (for CSV names)
    pub lst_cnt_upd: Cell<i32>,
    pub hooks: Vec<Option<ObjId>>,   // HOOK(i); new objects start with none, as in LSD
}

/// One value an equation read while being explained: what it asked for, where it was found, and what it got.
#[derive(Clone, Debug)]
pub struct Node {
    pub label: String,
    pub object: String, // instance path, as in CSV column names ("1_3")
    pub lag: i32,
    pub value: f64,
    pub computed: bool, // its equation ran because of this request
    pub children: Vec<Node>,
}

/// Records the tree of values behind one computation (`lsd explain`).
#[derive(Default)]
pub struct Explain {
    pub target: String,
    pub t: i32,
    pub instance: Option<String>,
    pub(crate) stack: Vec<Node>,
    pub result: Option<Node>,
}

/// A saved series of a deleted object (LSD's cemetery).
pub struct Dead {
    pub label: Rc<str>,
    pub lab_tit: String,
    pub data: Vec<f64>,
    pub start: i32,
    pub end: i32,
}

pub struct Model {
    pub objs: Vec<Option<Box<Object>>>, // deleted objects leave an empty slot; ids are never reused
    pub root: ObjId,
    blueprint: ObjId, // detached copy of the initial structure, one instance per type
    pub t: i32,
    pub max_step: i32,
    pub rng: LsdRng,
    eqs: FxMap<String, EqDef>,
    types: FxMap<Rc<str>, u32>,
    pub use_nan: bool,
    pub no_zero_instance: bool,
    running: bool,
    deleted: bool, // LSD's static `deleted` in object::update
    wait_delete: Option<ObjId>,
    pub cemetery: Vec<Dead>,
    trace: Option<BufWriter<File>>,
    pub no_search: bool,                       // NO_SEARCH: look only in the object itself
    pub abort: bool,                           // ABORT: stop after the current step
    pub no_saved: bool,                        // LSD default: lags beyond the buffer are not read from saved series (USE_SAVED)
    pub steps_done: i32,                       // LSD's actual_steps: rows in the results
    pub seed: u32,
    pub sim_name: String,                      // configuration name, as LSD's simul_name
    pub ext: HashMap<ObjId, Box<dyn std::any::Any>>, // EXT: a model's own data attached to an object
    pub nets: crate::nets::Nets,               // network nodes and links
    tinfo: Vec<TypeInfo>,
    pub explain: Option<Explain>,
    plans: RefCell<FxMap<(u32, usize, usize), Option<Plan>>>, // (type, label address, length)
}

/// What an equation sees: the model, its own object `p`, the caller `c`, and its variable.
pub struct Ctx<'a> {
    pub m: &'a mut Model,
    pub p: ObjId,
    pub c: Option<ObjId>,
    pub var: (ObjId, usize),
}

macro_rules! fail {
    ($($a:tt)*) => { panic!("LSD error: {}", format!($($a)*)) };
}

impl Model {
    pub fn new(cfg: &Config, eqs: &[(&'static str, EqDef)], seed: u32) -> Model {
        let mut m = Model {
            objs: Vec::new(),
            root: 0,
            blueprint: 0,
            t: 0,
            max_step: cfg.max_step as i32,
            rng: LsdRng::new(seed),
            eqs: eqs.iter().rev().map(|(l, e)| (l.to_string(), *e)).collect(), // the first definition of a label wins, as in LSD
            types: FxMap::default(),
            use_nan: false,
            no_zero_instance: true,
            running: false,
            deleted: false,
            wait_delete: None,
            cemetery: Vec::new(),
            trace: None,
            no_search: false,
            abort: false,
            no_saved: true,
            steps_done: 0,
            seed,
            sim_name: String::new(),
            ext: HashMap::new(),
            nets: Default::default(),
            tinfo: Vec::new(),
            explain: None,
            plans: RefCell::new(FxMap::default()),
        };
        // instances of each type, in tree order; `next_inst` hands them out in that order
        let mut layouts = HashMap::new();
        let mut next_inst: HashMap<String, usize> = HashMap::new();
        m.root = m.build(&cfg.root, None, 0, &mut layouts, &mut next_inst);
        m.blueprint = m.copy_first(m.root, None);
        m.tinfo = (0..m.types.len()).map(|_| TypeInfo { index: Rc::new(FxMap::default()), sons: Vec::new(), up: None }).collect();
        let bp = m.blueprint;
        m.fill_tinfo(bp, None);
        m
    }

    pub fn trace_to(&mut self, path: &str) {
        self.trace = Some(BufWriter::new(File::create(path).expect("cannot create trace file")));
    }

    fn build(&mut self, spec: &ObjSpec, up: Option<ObjId>, _k: usize,
             layouts: &mut HashMap<String, Rc<FxMap<Rc<str>, usize>>>, next_inst: &mut HashMap<String, usize>) -> ObjId {
        let inst = *next_inst.entry(spec.label.clone()).or_insert(0);
        next_inst.insert(spec.label.clone(), inst + 1);
        let index = layouts.entry(spec.label.clone()).or_insert_with(|| {
            Rc::new(spec.elems.iter().enumerate().map(|(i, e)| (Rc::from(e.label.as_str()), i)).collect())
        }).clone();
        let vars = spec.elems.iter().map(|e| {
            let mut val = vec![0.0; e.num_lag + 1];
            for (i, x) in e.values[inst].iter().enumerate() {
                val[i] = *x; // initial values are the "time 0" values
            }
            Var {
                label: Rc::from(e.label.as_str()),
                kind: e.kind,
                num_lag: e.num_lag,
                val,
                last_update: 0,
                next_update: 0,
                upd: e.upd.unwrap_or([0, 0, 0, 0]),
                save: e.save,
                data: Vec::new(),
                start: 0,
                end: 0,
                under_computation: false,
                dummy: false,
                eq: self.eqs.get(&e.label).copied(),
            }
        }).collect();
        let ty = self.ty_of_or_new(&spec.label);
        let sons = spec.sons.iter().map(|s| Bridge { label: Rc::from(s.label.as_str()), ty: self.ty_of_or_new(&s.label), counter_updated: Cell::new(false), head: None }).collect();
        let id = self.objs.len();
        self.objs.push(Some(Box::new(Object {
            label: Rc::from(spec.label.as_str()),
            ty,
            up,
            next: None,
            vars,
            index,
            sons,
            to_compute: spec.to_compute,
            in_update: false,
            deleting: false,
            hook: None,
            hooks: Vec::new(),
            acounter: Cell::new(0),
            lst_cnt_upd: Cell::new(0),
        })));
        for (b, son) in spec.sons.iter().enumerate() {
            // this parent's group of `son` instances: counts are per parent instance, in tree order
            let parent_inst = inst;
            let n = son.counts[parent_inst];
            let mut prev: Option<ObjId> = None;
            for _ in 0..n {
                let c = self.build(son, Some(id), 0, layouts, next_inst);
                match prev {
                    None => self.o_mut(id).sons[b].head = Some(c),
                    Some(pv) => self.o_mut(pv).next = Some(c),
                }
                prev = Some(c);
            }
        }
        id
    }

    /// Detached copy of `o` keeping only the first instance of each son type (LSD's set_blueprint).
    fn copy_first(&mut self, o: ObjId, up: Option<ObjId>) -> ObjId {
        let mut obj = self.o(o).clone();
        obj.up = up;
        obj.next = None;
        let id = self.objs.len();
        self.objs.push(None);
        let heads: Vec<Option<ObjId>> = obj.sons.iter().map(|b| b.head).collect();
        for (b, h) in heads.into_iter().enumerate() {
            obj.sons[b].head = h.map(|h| self.copy_first(h, Some(id)));
        }
        self.objs[id] = Some(Box::new(obj));
        id
    }

    pub fn o(&self, o: ObjId) -> &Object {
        match self.objs.get(o) {
            Some(Some(x)) => x,
            _ => fail!("object {} was deleted or never existed (dangling pointer)", o),
        }
    }

    fn o_mut(&mut self, o: ObjId) -> &mut Object {
        match self.objs.get_mut(o) {
            Some(Some(x)) => x,
            _ => fail!("object {} was deleted or never existed (dangling pointer)", o),
        }
    }

    pub fn exists(&self, o: ObjId) -> bool {
        matches!(self.objs.get(o), Some(Some(_)))
    }

    fn ty_of_or_new(&mut self, lab: &str) -> u32 {
        let n = self.types.len() as u32;
        *self.types.entry(Rc::from(lab)).or_insert(n)
    }

    /// The interned id of an object type, if the model has it.
    pub fn ty_of(&self, lab: &str) -> Option<u32> {
        self.types.get(lab).copied()
    }

    pub fn label(&self, o: ObjId) -> &str {
        &self.o(o).label
    }

    pub fn up(&self, o: ObjId) -> Option<ObjId> {
        self.o(o).up
    }

    pub fn next(&self, o: ObjId) -> Option<ObjId> {
        self.o(o).next
    }

    pub fn var(&self, r: (ObjId, usize)) -> &Var {
        &self.o(r.0).vars[r.1]
    }

    fn var_mut(&mut self, r: (ObjId, usize)) -> &mut Var {
        &mut self.o_mut(r.0).vars[r.1]
    }

    fn tr(&mut self, kind: &str, lab: &str, v: f64) {
        let t = self.t;
        if let Some(w) = self.trace.as_mut() {
            writeln!(w, "{}\t{}\t{}\t{}", t, kind, lab, fmt17(v).to_lowercase()).unwrap(); // C "%.17g"
        }
    }

    // ------------------------------------------------------------------ searching

    /// LSD object::search_var: own variables, then the first instance of each descendant type,
    /// then up the tree (never back into the branch the search came from).
    fn search_var(&self, o: ObjId, caller: Option<ObjId>, lab: &str, no_up: bool) -> Option<(ObjId, usize)> {
        let obj = self.o(o);
        if let Some(&i) = obj.index.get(lab) {
            return Some((o, i));
        }
        for b in &obj.sons {
            if let Some(h) = b.head {
                if caller.map_or(true, |c| self.o(h).ty != self.o(c).ty) {
                    if let Some(r) = self.search_var(h, Some(o), lab, true) {
                        return Some(r);
                    }
                }
            }
        }
        if no_up {
            return None;
        }
        match obj.up {
            Some(u) if Some(u) != caller => self.search_var(u, Some(o), lab, false),
            _ => None,
        }
    }

    fn fill_tinfo(&mut self, o: ObjId, up: Option<u32>) {
        let obj = self.o(o);
        let (ty, index) = (obj.ty as usize, obj.index.clone());
        let sons: Vec<(u32, Option<ObjId>)> = obj.sons.iter().map(|b| (b.ty, b.head)).collect();
        self.tinfo[ty] = TypeInfo { index, sons: sons.iter().map(|s| s.0).collect(), up };
        for (_, h) in sons {
            if let Some(h) = h {
                self.fill_tinfo(h, Some(ty as u32));
            }
        }
    }

    /// search_var on the type structure (every bridge assumed to have instances).
    fn plan_search(&self, ty: u32, caller: Option<u32>, lab: &str, no_up: bool, steps: &mut Vec<Step>) -> Option<usize> {
        let ti = &self.tinfo[ty as usize];
        if let Some(&i) = ti.index.get(lab) {
            return Some(i);
        }
        for (b, &son) in ti.sons.iter().enumerate() {
            if caller != Some(son) {
                steps.push(Step::Down(b));
                if let Some(i) = self.plan_search(son, Some(ty), lab, true, steps) {
                    return Some(i);
                }
                steps.pop();
            }
        }
        if no_up {
            return None;
        }
        let up = ti.up?;
        steps.push(Step::Up);
        let r = self.plan_search(up, Some(ty), lab, false, steps);
        if r.is_none() {
            steps.pop();
        }
        r
    }

    /// Follow a plan; None if a bridge on the way has no instances (then LSD's search may land elsewhere).
    fn follow(&self, o: ObjId, plan: &Plan) -> Option<(ObjId, usize)> {
        let mut cur = o;
        for st in &plan.steps {
            cur = match *st {
                Step::Up => self.o(cur).up?,
                Step::Down(b) => self.o(cur).sons[b].head?,
            };
        }
        Some((cur, plan.var))
    }

    pub fn find_var(&self, o: ObjId, lab: &str) -> (ObjId, usize) {
        let found = if self.no_search {
            self.o(o).index.get(lab).map(|&i| (o, i))
        } else {
            let ty = self.o(o).ty;
            let key = (ty, lab.as_ptr() as usize, lab.len());
            let hit = match self.plans.borrow().get(&key) {
                Some(Some(p)) if *p.label == *lab => Some(self.follow(o, p)),
                Some(None) => Some(None),
                _ => None,
            };
            let r = match hit {
                Some(r) => r,
                None => {
                    let mut steps = Vec::new();
                    let p = self.plan_search(ty, Some(ty), lab, false, &mut steps).map(|var| Plan { label: Rc::from(lab), steps, var });
                    let r = p.as_ref().and_then(|p| self.follow(o, p));
                    self.plans.borrow_mut().insert(key, p);
                    r
                }
            };
            r.or_else(|| self.search_var(o, Some(o), lab, false))
        };
        found
            .unwrap_or_else(|| fail!("element '{}' not found from object '{}'", lab, self.label(o)))
    }

    fn own_var(&self, o: ObjId, lab: &str) -> (ObjId, usize) {
        match self.o(o).index.get(lab) {
            Some(&i) => (o, i),
            None => fail!("element '{}' is not in object '{}'", lab, self.label(o)),
        }
    }

    /// LSD object::search(lab, no_search = false, no_search_up): self, direct sons, descendants, then up.
    fn search_obj(&self, o: ObjId, lab: &str, up: bool) -> Option<ObjId> {
        self.search_ty(o, self.ty_of(lab)?, up)
    }

    fn search_ty(&self, o: ObjId, ty: u32, up: bool) -> Option<ObjId> {
        let obj = self.o(o);
        if obj.ty == ty {
            return Some(o);
        }
        if let Some(b) = obj.sons.iter().find(|b| b.ty == ty) {
            return b.head;
        }
        for b in &obj.sons {
            if let Some(h) = b.head {
                if let Some(r) = self.search_ty(h, ty, false) {
                    return Some(r);
                }
            }
        }
        match (up, obj.up) {
            (true, Some(u)) => self.search_ty(u, ty, true),
            _ => None,
        }
    }

    /// SEARCH / SEARCHS
    pub fn search(&self, o: ObjId, lab: &str) -> Option<ObjId> {
        self.search_obj(o, lab, false)
    }

    /// First object of a CYCLE (cycle_obj: search with up-search enabled).
    pub fn cycle_first(&self, o: ObjId, lab: &str) -> Option<ObjId> {
        let r = self.search_obj(o, lab, true);
        if r.is_none() && self.no_zero_instance {
            fail!("object '{}' not found for cycling from '{}'", lab, self.label(o));
        }
        r
    }

    /// Instances from `first` following `next` (a snapshot; use `next` directly for live cycling).
    pub fn brothers(&self, first: Option<ObjId>) -> Vec<ObjId> {
        let mut v = Vec::new();
        let mut c = first;
        while let Some(x) = c {
            v.push(x);
            c = self.o(x).next;
        }
        v
    }

    /// LSD hyper_next: next instance of the same type anywhere in the model.
    pub fn hyper_next(&self, o: ObjId) -> Option<ObjId> {
        self.hyper_next_ty(o, self.o(o).ty)
    }

    fn hyper_next_ty(&self, o: ObjId, ty: u32) -> Option<ObjId> {
        let mut cur = self.o(o).next;
        while let Some(c) = cur {
            if let Some(r) = self.search_ty(c, ty, false) {
                return Some(r);
            }
            cur = self.skip_next_obj(c);
        }
        self.o(o).up.and_then(|u| self.hyper_next_ty(u, ty))
    }

    /// First instance of the bridge after the one containing `o` in its parent.
    fn skip_next_obj(&self, o: ObjId) -> Option<ObjId> {
        let up = self.o(o).up?;
        let sons = &self.o(up).sons;
        let b = sons.iter().position(|b| b.ty == self.o(o).ty)?;
        sons.get(b + 1).and_then(|b| b.head)
    }

    // ------------------------------------------------------------------ evaluation

    /// object::cal: find `lab` from `o` and return its value, computing it if needed.
    pub fn cal(&mut self, o: ObjId, caller: Option<ObjId>, lab: &str, lag: i32) -> f64 {
        let r = self.find_var(o, lab);
        if self.explain.as_ref().map_or(false, |e| !e.stack.is_empty()) {
            return self.cal_recorded(r, caller, lab, lag);
        }
        self.cal_var(r, caller, lag)
    }

    fn cal_recorded(&mut self, r: (ObjId, usize), caller: Option<ObjId>, lab: &str, lag: i32) -> f64 {
        let object = self.lab_tit(r.0);
        let before = self.var(r).last_update;
        self.explain.as_mut().unwrap().stack.push(Node { label: lab.into(), object, lag, value: f64::NAN, computed: false, children: vec![] });
        let v = self.cal_var(r, caller, lag);
        let ex = self.explain.as_mut().unwrap();
        let mut n = ex.stack.pop().unwrap();
        n.value = v;
        n.computed = lag == 0 && before < self.t && self.var(r).last_update == self.t;
        if let Some(parent) = self.explain.as_mut().unwrap().stack.last_mut() {
            parent.children.push(n);
        }
        v
    }

    /// variable::cal
    pub fn cal_var(&mut self, r: (ObjId, usize), caller: Option<ObjId>, lag: i32) -> f64 {
        let t = self.t;
        let v = self.var(r);
        let num_lag = v.num_lag as i32;
        match v.kind {
            Kind::Param => return v.val[0],
            Kind::Var => {
                if lag > num_lag && (self.no_saved || !v.save || t - lag < v.start) {
                    fail!("lag {} of '{}' is beyond its {} stored lags (saved values are read only after USE_SAVED, and only if saved)", lag, v.label, num_lag);
                }
                if lag != 0 {
                    let eff_lag = if v.last_update < t { lag - 1 } else { lag };
                    if eff_lag < 0 {
                        fail!("negative lag for '{}'", v.label);
                    }
                    if eff_lag > num_lag {
                        if self.no_saved || !v.save || lag > t - v.start {
                            fail!("lag {} of '{}' is beyond its {} stored lags and not saved", lag, v.label, num_lag);
                        }
                        return v.data[(t - lag - v.start) as usize];
                    }
                    return v.val[eff_lag as usize];
                }
                if v.last_update >= t || t < v.next_update {
                    return v.val[0];
                }
            }
            Kind::Func => {
                if lag < 0 || lag > num_lag {
                    fail!("invalid lag {} for function '{}'", lag, v.label);
                }
                if lag > 0 {
                    return v.val[lag as usize - 1];
                }
                if caller.is_none() {
                    return v.val[0];
                }
            }
        }
        if v.under_computation {
            fail!("deadlock: equation for '{}' (object '{}') requested its own value while computing it", v.label, self.label(r.0));
        }
        let label = v.label.clone();
        self.var_mut(r).under_computation = true;
        let explaining = self.explain.as_ref().map_or(false, |e| {
            e.result.is_none() && e.stack.is_empty() && e.t == t && *e.target == *label
        }) && {
            let want = self.explain.as_ref().unwrap().instance.clone();
            want.map_or(true, |w| w == self.lab_tit(r.0))
        };
        if explaining {
            let object = self.lab_tit(r.0);
            self.explain.as_mut().unwrap().stack.push(Node { label: label.to_string(), object, lag: 0, value: f64::NAN, computed: true, children: vec![] });
        }
        let app = self.run_equation(r, caller, &label);
        if explaining {
            let ex = self.explain.as_mut().unwrap();
            let mut n = ex.stack.pop().unwrap();
            n.value = app;
            ex.result = Some(n);
        }
        let v = self.var_mut(r);
        for i in 0..v.num_lag {
            v.val[v.num_lag - i] = v.val[v.num_lag - i - 1];
        }
        v.val[0] = app;
        v.last_update = t;
        let (period, period_range) = (v.upd[2], v.upd[3]);
        if period > 1 || period_range > 0 {
            let extra = if period_range > 0 { self.rng.rnd_int(0, period_range) } else { 0 };
            self.var_mut(r).next_update = t + period + extra;
        }
        self.tr("c", &label, app);
        self.var_mut(r).under_computation = false;
        if let Some(w) = self.wait_delete {
            self.delete(w);
        }
        self.var(r).val[0]
    }

    fn run_equation(&mut self, r: (ObjId, usize), caller: Option<ObjId>, label: &str) -> f64 {
        let eq = match self.var(r).eq {
            Some(e) => e,
            None => fail!("equation not found for variable '{}'", label),
        };
        let res = match eq {
            EqDef::Fn(f) => f(&mut Ctx { m: self, p: r.0, c: caller, var: r }),
            EqDef::Function(f) => {
                // LSD's FUNCTION macro: last_update-- (overwritten by cal), stored value when not requested
                self.var_mut(r).last_update -= 1;
                if caller.is_none() {
                    return self.var(r).val[0];
                }
                f(&mut Ctx { m: self, p: r.0, c: caller, var: r })
            }
            EqDef::Hook(_) => fail!("'{}' is a hook, not an equation", label),
            EqDef::Dummy(other) => {
                // EQUATION_DUMMY(X, Y): X is written by Y's equation; Y may be "" (nothing to compute)
                let o = r.0;
                if !other.is_empty() {
                    let ro = self.find_var(o, other);
                    if !self.var(ro).under_computation {
                        self.var_mut(r).dummy = true;
                        self.cal_var(ro, Some(o), 0);
                    }
                }
                self.var(r).val[0]
            }
        };
        // LSD skips this check once ABORT is set (quit != 0)
        if !self.abort && ((!self.use_nan && res.is_nan()) || res.is_infinite()) {
            fail!("equation for '{}' produces the invalid value '{}' at case {}", label, res, self.t);
        }
        res
    }

    /// object::write(lab, value, time, lag): only the object's own element.
    pub fn write(&mut self, o: ObjId, lab: &str, value: f64, time: i32, lag: i32) -> f64 {
        if (!self.use_nan && value.is_nan()) || value.is_infinite() {
            fail!("value '{}' is invalid for writing to element '{}'", value, lab);
        }
        let r = self.own_var(o, lab);
        let t = self.t;
        let v = self.var(r);
        if v.under_computation && !v.dummy {
            fail!("variable '{}' is under computation and cannot be written", lab);
        }
        if v.kind != Kind::Param && time <= 0 && t > 1 {
            fail!("invalid update time ({}) for variable '{}'", time, lab);
        }
        if v.kind != Kind::Param && time < 0 && t == 1 {
            // change of initial lagged values when starting
            if (-time) as usize > v.num_lag {
                fail!("invalid initial lag ({}) for variable '{}'", time, lab);
            }
            let v = self.var_mut(r);
            v.val[(-time - 1) as usize] = value;
            v.last_update = 0;
            if time == -1 && v.save {
                v.data[0] = value;
            }
            let (delay, delay_range) = (v.upd[0], v.upd[1]);
            if delay > 0 || delay_range > 0 {
                let extra = if delay_range > 0 { self.rng.rnd_int(0, delay_range) } else { 0 };
                self.var_mut(r).next_update = delay + extra;
            }
            return value;
        }
        if lag < 0 || (v.kind != Kind::Param && lag as usize > v.num_lag) || (v.kind == Kind::Param && lag > 1) {
            fail!("invalid lag ({}) for '{}'", lag, lab);
        }
        let (eff_lag, eff_time);
        if v.kind == Kind::Param {
            eff_lag = 0;
            eff_time = time;
        } else {
            if time >= t && lag == 0 && v.last_update < t {
                let v = self.var_mut(r);
                for i in 0..v.num_lag {
                    v.val[v.num_lag - i] = v.val[v.num_lag - i - 1];
                }
            }
            let v = self.var(r);
            if lag == 0 {
                eff_lag = 0;
                eff_time = time;
                let (period, period_range) = (v.upd[2], v.upd[3]);
                if period > 1 || period_range > 0 {
                    let extra = if period_range > 0 { self.rng.rnd_int(0, period_range) } else { 0 };
                    self.var_mut(r).next_update = t + period + extra;
                }
            } else {
                let v = self.var(r);
                if time >= t || time >= v.last_update {
                    eff_lag = lag - (t - v.last_update);
                    eff_time = time - lag;
                } else {
                    eff_lag = lag - (t - time);
                    eff_time = t - lag;
                }
                if eff_lag < 0 || eff_lag as usize > v.num_lag {
                    fail!("invalid update case ({}) and lag ({}) for variable '{}'", time, lag, lab);
                }
            }
        }
        let v = self.var_mut(r);
        v.val[eff_lag as usize] = value;
        v.last_update = time;
        if v.save {
            if eff_time >= v.start && eff_time <= v.end {
                let i = (eff_time - v.start) as usize;
                v.data[i] = value;
            } else if time == 0 && v.start == 0 {
                v.data[0] = value;
            }
        }
        self.tr("w", lab, value);
        value
    }

    /// INCR / MULT
    pub fn increment(&mut self, o: ObjId, lab: &str, value: f64, mult: bool) -> f64 {
        if (!self.use_nan && value.is_nan()) || value.is_infinite() {
            fail!("value '{}' is invalid for changing element '{}'", value, lab);
        }
        let r = self.own_var(o, lab);
        if !self.use_nan && self.var(r).val[0].is_nan() {
            self.cal_var(r, Some(o), 0);
        }
        let cur = self.var(r).val[0];
        let new = if mult { cur * value } else { cur + value };
        let t = self.t;
        self.write(o, lab, new, t, 0)
    }

    // ------------------------------------------------------------------ objects

    /// ADDOBJ_EX: add `n` copies of `ex` under `parent`, at the end of its `lab` instances.
    pub fn add_objects(&mut self, parent: ObjId, lab: &str, n: usize, ex: Option<ObjId>, t_update: i32) -> ObjId {
        let b = match self.o(parent).sons.iter().position(|b| *b.label == *lab) {
            Some(b) => b,
            None => fail!("object '{}' contains no son object '{}' for adding instance(s)", self.label(parent), lab),
        };
        let ex = match ex {
            Some(e) => e,
            None => self.search_obj(self.blueprint, lab, false).unwrap_or_else(|| fail!("'{}' not in blueprint", lab)),
        };
        if *self.o(ex).label != *lab {
            fail!("bad example pointer when adding object '{}'", lab);
        }
        let mut first = None;
        let mut last = None;
        for _ in 0..n {
            let c = self.copy_object(ex, parent, t_update);
            match last {
                None => {
                    first = Some(c);
                    match self.o(parent).sons[b].head {
                        None => self.o_mut(parent).sons[b].head = Some(c),
                        Some(h) => {
                            let mut x = h;
                            while let Some(nx) = self.o(x).next {
                                x = nx;
                            }
                            self.o_mut(x).next = Some(c);
                        }
                    }
                }
                Some(l) => self.o_mut(l).next = Some(c),
            }
            last = Some(c);
        }
        self.o(parent).sons[b].counter_updated.set(false);
        first.unwrap()
    }

    fn copy_object(&mut self, ex: ObjId, parent: ObjId, t_update: i32) -> ObjId {
        let t = self.t;
        let src = self.o(ex).clone();
        let mut vars = src.vars.clone();
        for v in vars.iter_mut() {
            v.under_computation = false;
            v.dummy = false;
            v.next_update = 0;
            if self.running && v.kind != Kind::Param {
                if t_update < 0 && v.last_update == 0 {
                    v.last_update = t;
                } else if t_update >= 0 {
                    v.last_update = t_update;
                }
                let (delay, delay_range) = (v.upd[0], v.upd[1]);
                if delay > 0 || delay_range > 0 {
                    v.next_update = v.last_update + delay;
                    if delay_range > 0 {
                        v.next_update += self.rng.rnd_int(0, delay_range);
                    }
                }
            }
            if v.save && self.running {
                alloc_save(v, t, self.max_step);
            }
        }
        let id = self.objs.len();
        self.objs.push(Some(Box::new(Object {
            label: src.label.clone(),
            ty: src.ty,
            up: Some(parent),
            next: None,
            vars,
            index: src.index.clone(),
            sons: src.sons.iter().map(|b| Bridge { label: b.label.clone(), ty: b.ty, counter_updated: Cell::new(false), head: None }).collect(),
            to_compute: src.to_compute,
            in_update: false,
            deleting: false,
            hook: None,
            hooks: Vec::new(),
            acounter: Cell::new(0),
            lst_cnt_upd: Cell::new(0),
        })));
        for b in &src.sons {
            for c in self.brothers(b.head) {
                self.add_objects(id, &b.label, 1, Some(c), t_update);
            }
        }
        crate::nets::object_added(self, ex, id);
        id
    }

    fn under_computation(&self, o: ObjId) -> bool {
        let obj = self.o(o);
        obj.sons.iter().any(|b| self.brothers(b.head).into_iter().any(|c| self.under_computation(c)))
            || obj.vars.iter().any(|v| v.under_computation && !v.dummy)
    }

    /// DELETE (object::delete_obj): deferred while the object is under computation.
    pub fn delete(&mut self, o: ObjId) {
        if !self.exists(o) || self.o(o).deleting {
            return;
        }
        if self.under_computation(o) {
            match self.wait_delete {
                Some(w) if w != o => fail!("cannot schedule the deletion of object '{}'", self.label(o)),
                _ => {
                    self.wait_delete = Some(o);
                    return;
                }
            }
        }
        self.o_mut(o).deleting = true;
        if self.wait_delete == Some(o) {
            self.wait_delete = None;
        }
        self.collect_cemetery(o);
        let up = self.o(o).up.unwrap();
        let lab = self.o(o).label.clone();
        let ty = self.o(o).ty;
        let b = self.o(up).sons.iter().position(|b| b.ty == ty).unwrap();
        let head = self.o(up).sons[b].head.unwrap();
        let nx = self.o(o).next;
        if head == o {
            if nx.is_none() && self.no_zero_instance {
                fail!("cannot delete all instances of '{}'", lab);
            }
            self.o_mut(up).sons[b].head = nx;
        } else {
            let mut x = head;
            while self.o(x).next != Some(o) {
                x = self.o(x).next.unwrap();
            }
            self.o_mut(x).next = nx;
        }
        self.o(up).sons[b].counter_updated.set(false);
        if self.o(o).in_update {
            self.deleted = true;
        }
        self.drop_tree(o);
    }

    fn collect_cemetery(&mut self, o: ObjId) {
        let t = self.t;
        if self.running && t > 0 {
            for i in 0..self.o(o).vars.len() {
                if self.o(o).vars[i].save {
                    let lab_tit = self.lab_tit(o);
                    let v = self.var_mut((o, i));
                    v.end = t;
                    let k = (t - v.start) as usize;
                    v.data[k] = v.val[0];
                    v.data.truncate(k + 1);
                    let d = Dead { label: v.label.clone(), lab_tit, data: std::mem::take(&mut v.data), start: v.start, end: t };
                    self.cemetery.push(d);
                }
            }
        }
    }

    /// Remove `o` and its descendants; each son's saved series go to the cemetery first (LSD's ~bridge order).
    fn drop_tree(&mut self, o: ObjId) {
        for b in self.o(o).sons.clone() {
            for c in self.brothers(b.head) {
                self.collect_cemetery(c);
                self.drop_tree(c);
            }
        }
        crate::nets::object_deleted(self, o);
        self.ext.remove(&o);
        self.objs[o] = None;
    }

    /// LSD set_counter: refresh the cached positions of `o`'s brothers (and ancestors), at most once per step
    /// for each instance, and only if the bridge changed since (additions, deletions and sorts reset it).
    fn set_counter(&self, o: ObjId) {
        let Some(up) = self.o(o).up else { return };
        self.set_counter(up);
        let ty = self.o(o).ty;
        let Some(b) = self.o(up).sons.iter().find(|b| b.ty == ty) else { return };
        if b.counter_updated.get() {
            return;
        }
        for (i, c) in self.brothers(b.head).into_iter().enumerate() {
            let x = self.o(c);
            if x.lst_cnt_upd.get() < self.t {
                x.acounter.set(i as i32 + 1);
                x.lst_cnt_upd.set(self.t);
            }
        }
        b.counter_updated.set(true);
    }

    /// Mark the instance positions of `lab` under `parent` as changed (after sorting them).
    pub fn reset_counter(&self, parent: ObjId, ty: u32) {
        if let Some(b) = self.o(parent).sons.iter().find(|b| b.ty == ty) {
            b.counter_updated.set(false);
        }
    }

    /// LSD set_lab_tit: instance positions from the top level down, e.g. "1_3", from the cached counters.
    pub fn lab_tit(&self, o: ObjId) -> String {
        if self.o(o).up.is_none() {
            return "R".into();
        }
        self.set_counter(o);
        let mut parts = Vec::new();
        let mut cur = o;
        while let Some(u) = self.o(cur).up {
            parts.push(self.o(cur).acounter.get().to_string());
            cur = u;
        }
        parts.reverse();
        parts.join("_")
    }

    // ------------------------------------------------------------------ running

    /// One simulation: t = 1..=max_step, each step `root.update()`.
    pub fn run(&mut self) {
        self.running = true;
        self.t = 1;
        let all: Vec<ObjId> = (0..self.objs.len()).filter(|&o| self.exists(o) && self.attached(o)).collect();
        let (t, max_step) = (self.t, self.max_step);
        for o in all {
            for v in self.o_mut(o).vars.iter_mut() {
                if v.save {
                    alloc_save(v, t, max_step);
                }
            }
        }
        for t in 1..=self.max_step {
            self.t = t;
            let root = self.root;
            self.update(root);
            self.steps_done = t;
            if self.abort {
                break;
            }
        }
        self.t = self.steps_done + 1;
        if let Some(EqDef::Hook(f)) = self.eqs.get("close_sim").copied() {
            f(self);
        }
        if let Some(w) = self.trace.as_mut() {
            w.flush().unwrap();
        }
    }

    fn attached(&self, mut o: ObjId) -> bool {
        while let Some(u) = self.o(o).up {
            o = u;
        }
        o == self.root
    }

    /// object::update: own variables in order, then sons, each instance in order.
    pub fn update(&mut self, o: ObjId) {
        self.update_obj(o, true)
    }

    /// object::update(recurse): also what UPDATE (recurse = false) and UPDATE_REC do inside an equation.
    pub fn update_obj(&mut self, o: ObjId, recurse: bool) {
        self.deleted = false;
        self.o_mut(o).in_update = true;
        let t = self.t;
        let n = self.o(o).vars.len();
        for i in 0..n {
            if self.deleted {
                break;
            }
            let v = &self.o(o).vars[i];
            if v.under_computation {
                continue;
            }
            if v.kind == Kind::Var && v.last_update < t {
                self.cal_var((o, i), None, 0);
            }
            if !self.deleted {
                let v = self.var_mut((o, i));
                if v.save {
                    let k = (t - v.start) as usize;
                    v.data[k] = v.val[0];
                }
            }
        }
        let nb = if self.deleted || !recurse { 0 } else { self.o(o).sons.len() };
        for b in 0..nb {
            if self.deleted {
                break;
            }
            let head = self.o(o).sons[b].head;
            if let Some(h) = head {
                if self.o(h).to_compute {
                    let mut cur = Some(h);
                    while let Some(c) = cur {
                        if self.deleted {
                            break;
                        }
                        let nx = self.o(c).next;
                        self.update_obj(c, true);
                        cur = nx;
                    }
                }
            }
        }
        if !self.deleted {
            self.o_mut(o).in_update = false;
        }
    }
}

/// alloc_save_var: lagged variables and parameters keep their time-0 value.
fn alloc_save(v: &mut Var, t: i32, max_step: i32) {
    v.start = if v.num_lag > 0 || v.kind == Kind::Param { t - 1 } else { t };
    v.end = max_step;
    v.data = vec![f64::NAN; (v.end - v.start + 1) as usize];
    if v.num_lag > 0 || v.kind == Kind::Param {
        v.data[0] = v.val[0];
    }
}

/// C's "%.17G" for doubles, as LSD writes them.
pub fn fmt17(v: f64) -> String {
    let mut s = String::new();
    fmt17_into(&mut s, v);
    s
}

/// Append C's "%.17G" of `v` to `out`: 17 significant digits, fixed or exponent form, trailing zeros removed.
pub fn fmt17_into(out: &mut String, v: f64) {
    use std::fmt::Write as _;
    if v.is_nan() {
        out.push_str("NAN");
        return;
    }
    if v.is_infinite() {
        out.push_str(if v > 0.0 { "INF" } else { "-INF" });
        return;
    }
    if v == 0.0 {
        out.push_str(if v.is_sign_negative() { "-0" } else { "0" });
        return;
    }
    // Rust's "{:.16e}" rounds to 17 significant digits exactly like printf; write it to a stack buffer
    struct Stack { b: [u8; 40], n: usize }
    impl std::fmt::Write for Stack {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            self.b[self.n..self.n + s.len()].copy_from_slice(s.as_bytes());
            self.n += s.len();
            Ok(())
        }
    }
    let mut st = Stack { b: [0; 40], n: 0 };
    write!(st, "{:.16e}", v.abs()).unwrap(); // d.dddddddddddddddde±x
    let txt = std::str::from_utf8(&st.b[..st.n]).unwrap();
    let (mant, exp) = txt.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let mut dg = [0u8; 17];
    dg[0] = mant.as_bytes()[0];
    dg[1..].copy_from_slice(&mant.as_bytes()[2..18]);
    let digits = std::str::from_utf8(&dg).unwrap();
    if v < 0.0 {
        out.push('-');
    }
    if exp < -4 || exp >= 17 {
        let d = digits.trim_end_matches('0');
        out.push_str(&d[..1]);
        if d.len() > 1 {
            out.push('.');
            out.push_str(&d[1..]);
        }
        write!(out, "E{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs()).unwrap();
    } else if exp < 0 {
        out.push_str("0.");
        for _ in 0..(-exp - 1) {
            out.push('0');
        }
        out.push_str(digits.trim_end_matches('0'));
    } else {
        let int = (exp + 1) as usize;
        out.push_str(&digits[..int]);
        let frac = digits[int..].trim_end_matches('0');
        if !frac.is_empty() {
            out.push('.');
            out.push_str(frac);
        }
    }
}

// ------------------------------------------------------------------ the equation API (LSD macros)

impl<'a> Ctx<'a> {
    pub fn t(&self) -> f64 { self.m.t as f64 }
    /// PARAMETER: from now on this variable is a parameter (keeps its value)
    pub fn parameter(&mut self) { let r = self.var; self.m.objs[r.0].as_mut().unwrap().vars[r.1].kind = Kind::Param }
    /// CURRENT: the variable's value before this computation
    pub fn current(&self) -> f64 { self.m.var(self.var).val[0] }
    /// V(lab)
    pub fn v(&mut self, lab: &str) -> f64 { let p = self.p; self.m.cal(p, Some(p), lab, 0) }
    /// VL(lab, lag)
    pub fn vl(&mut self, lab: &str, lag: i32) -> f64 { let p = self.p; self.m.cal(p, Some(p), lab, lag) }
    /// VS(o, lab)
    pub fn vs(&mut self, o: ObjId, lab: &str) -> f64 { self.m.cal(o, Some(o), lab, 0) }
    /// VLS(o, lab, lag)
    pub fn vls(&mut self, o: ObjId, lab: &str, lag: i32) -> f64 { self.m.cal(o, Some(o), lab, lag) }
    /// V_CHEAT(lab, caller)
    pub fn v_cheat(&mut self, lab: &str, caller: ObjId) -> f64 { let p = self.p; self.m.cal(p, Some(caller), lab, 0) }
    /// WRITE / WRITES
    pub fn write(&mut self, lab: &str, x: f64) -> f64 { let (p, t) = (self.p, self.m.t); self.m.write(p, lab, x, t, 0) }
    pub fn writes(&mut self, o: ObjId, lab: &str, x: f64) -> f64 { let t = self.m.t; self.m.write(o, lab, x, t, 0) }
    /// WRITEL / WRITELS (value as if computed at `time`)
    pub fn writel(&mut self, lab: &str, x: f64, time: f64) -> f64 { let p = self.p; self.m.write(p, lab, x, time as i32, 0) }
    pub fn writels(&mut self, o: ObjId, lab: &str, x: f64, time: f64) -> f64 { self.m.write(o, lab, x, time as i32, 0) }
    /// INCR / INCRS / MULT / MULTS
    pub fn incr(&mut self, lab: &str, x: f64) -> f64 { let p = self.p; self.m.increment(p, lab, x, false) }
    pub fn incrs(&mut self, o: ObjId, lab: &str, x: f64) -> f64 { self.m.increment(o, lab, x, false) }
    pub fn mult(&mut self, lab: &str, x: f64) -> f64 { let p = self.p; self.m.increment(p, lab, x, true) }
    pub fn mults(&mut self, o: ObjId, lab: &str, x: f64) -> f64 { self.m.increment(o, lab, x, true) }
    /// First instance of a CYCLE over `lab` from p (follow it with `next`)
    pub fn first(&self, lab: &str) -> Option<ObjId> { self.m.cycle_first(self.p, lab) }
    pub fn firsts(&self, o: ObjId, lab: &str) -> Option<ObjId> { self.m.cycle_first(o, lab) }
    pub fn next(&self, o: ObjId) -> Option<ObjId> { self.m.next(o) }
    /// SEARCH / SEARCHS
    pub fn search(&self, lab: &str) -> Option<ObjId> { self.m.search(self.p, lab) }
    pub fn searchs(&self, o: ObjId, lab: &str) -> Option<ObjId> { self.m.search(o, lab) }
    /// SEARCH_CND / SEARCH_CNDS (object::search_var_cond)
    pub fn search_cnd(&mut self, lab: &str, value: f64) -> Option<ObjId> { let p = self.p; self.search_cnds(p, lab, value) }
    pub fn search_cnds(&mut self, o: ObjId, lab: &str, value: f64) -> Option<ObjId> {
        let (mut cur, _) = self.m.find_var(o, lab);
        loop {
            let nx = if self.m.no_search { self.m.next(cur) } else { self.m.hyper_next(cur) };
            if self.m.cal(cur, Some(cur), lab, 0) == value {
                return Some(cur);
            }
            cur = nx?;
        }
    }
    /// RNDDRAW / RNDDRAWS: an instance with probability proportional to lv; RNDDRAW_FAIR: equal probabilities
    pub fn rnddraw(&mut self, lo: &str, lv: &str) -> Option<ObjId> { let p = self.p; self.rnddrawls(p, lo, lv, 0) }
    pub fn rnddraws(&mut self, o: ObjId, lo: &str, lv: &str) -> Option<ObjId> { self.rnddrawls(o, lo, lv, 0) }
    pub fn rnddraw_fair(&mut self, lab: &str) -> Option<ObjId> { let p = self.p; self.rnddraw_fairs(p, lab) }
    /// ADDOBJ / ADDOBJS (blueprint) and ADDOBJ_EX / ADDOBJS_EX (copy of an example)
    pub fn addobj(&mut self, lab: &str) -> ObjId { let p = self.p; self.m.add_objects(p, lab, 1, None, -1) }
    pub fn addobjs(&mut self, o: ObjId, lab: &str) -> ObjId { self.m.add_objects(o, lab, 1, None, -1) }
    pub fn addobj_ex(&mut self, lab: &str, ex: ObjId) -> ObjId { let p = self.p; self.m.add_objects(p, lab, 1, Some(ex), -1) }
    pub fn addobjs_ex(&mut self, o: ObjId, lab: &str, ex: ObjId) -> ObjId { self.m.add_objects(o, lab, 1, Some(ex), -1) }
    /// DELETE
    pub fn delete(&mut self, o: ObjId) { self.m.delete(o) }
    /// RND and the distributions
    pub fn rnd(&mut self) -> f64 { self.m.rng.ran1() }
    pub fn uniform(&mut self, a: f64, b: f64) -> f64 { self.m.rng.uniform(a, b) }
    pub fn uniform_int(&mut self, a: f64, b: f64) -> f64 { self.m.rng.uniform_int(a, b) }
    pub fn norm(&mut self, mean: f64, sd: f64) -> f64 { self.m.rng.norm(mean, sd) }
    pub fn lnorm(&mut self, mean: f64, sd: f64) -> f64 { self.m.rng.lnorm(mean, sd) }
    pub fn gamma(&mut self, a: f64, b: f64) -> f64 { self.m.rng.gamma(a, b) }
    pub fn beta(&mut self, a: f64, b: f64) -> f64 { self.m.rng.beta(a, b) }
    pub fn poisson(&mut self, mean: f64) -> f64 { self.m.rng.poisson(mean) }
    pub fn bernoulli(&mut self, p: f64) -> f64 { self.m.rng.bernoulli(p) }
    /// THIS, PARENT, CALLER
    pub fn parent(&self) -> ObjId { self.m.up(self.p).expect("Root has no parent") }
    pub fn up(&self, o: ObjId) -> ObjId { self.m.up(o).expect("Root has no parent") }
    pub fn use_nan(&mut self) { self.m.use_nan = true }
    /// USE_SAVED / NO_SAVED: read lags beyond the buffer from saved series
    pub fn use_saved(&mut self) { self.m.no_saved = false }
    pub fn no_saved(&mut self) { self.m.no_saved = true }
    /// LOG / PLOG
    pub fn log(&self, s: &str) { eprint!("{}", s) }
}

#[cfg(test)]
mod tests {
    use super::fmt17;

    #[test]
    fn fmt17_matches_printf() {
        // values checked against C printf("%.17G")
        assert_eq!(fmt17(9.9436961646053099), "9.9436961646053099");
        assert_eq!(fmt17(18.59484339196862), "18.59484339196862");
        assert_eq!(fmt17(-293.56736043186606), "-293.56736043186606");
        assert_eq!(fmt17(1e6), "1000000");
        assert_eq!(fmt17(0.1), "0.10000000000000001");
        assert_eq!(fmt17(1e-5), "1.0000000000000001E-05");
        assert_eq!(fmt17(1e20), "1E+20");
        assert_eq!(fmt17(0.0), "0");
        assert_eq!(fmt17(-0.000123), "-0.00012300000000000001"); // expected values from C printf
        assert_eq!(fmt17(123456789012345678.0), "1.2345678901234568E+17");
        assert_eq!(fmt17(12345678901234567.0), "12345678901234568");
        assert_eq!(fmt17(-2.5), "-2.5");
    }
}
