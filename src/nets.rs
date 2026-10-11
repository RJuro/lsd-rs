//! Networks of objects (LSD 8.1 nets.cpp): nodes, weighted directed links, the network generators,
//! statistics, shuffling, Pajek files, and the `*_NET`, `*_NODE`, `*_LINK` macros as `Ctx` methods.
//!
//! LSD keeps a `netNode` in each node object and a doubly linked list of `netLink`s inside it. Here node
//! data lives in `Nets` keyed by `ObjId`, and links live in an arena addressed by `LinkId`, with the same
//! `first`/`last`/`next`/`prev` structure, so iteration order and behaviour under modification match.
//! Same draws in the same order, same iteration orders, same floating-point operations as nets.cpp.
//!
//! Known deviations (all from undefined behaviour in LSD, not from logic):
//! - Deleting an *object* removes its node and outgoing links only; links from other nodes stay and dangle,
//!   as in LSD (using them panics here, LSD reads freed memory).
//! - After the run LSD calls the model's `close_sim()`; the engine has no such hook, see `models::nets::close_sim`.
//! - `lsdqsort` by node id uses a stable sort; LSD's `qsort` order for equal ids is unspecified.

use std::collections::HashMap;
use std::io::Write as _;

use crate::model::{Ctx, Model, ObjId};

macro_rules! fail {
    ($($a:tt)*) => { panic!("LSD error: {}", format!($($a)*)) };
}

pub type LinkId = usize;

/// LSD `netNode`: the network data of a node object.
#[derive(Clone, Debug)]
pub struct Node {
    pub name: Option<String>,
    pub prob: f64,     // draw probability
    pub time: i32,     // creation time
    pub id: i64,       // reorderable id
    pub n_links: i64,  // number of outgoing links
    pub ser_num: i64,  // serial number (fixed)
    pub first: Option<LinkId>,
    pub last: Option<LinkId>,
}

/// LSD `netLink`: one outgoing link.
#[derive(Clone, Debug)]
pub struct Link {
    pub prob_to: f64,
    pub weight: f64,
    pub time: i32,
    pub ser_to: i64,
    pub next: Option<LinkId>,
    pub prev: Option<LinkId>,
    pub from: ObjId,
    pub to: ObjId,
    pub alive: bool,
}

pub struct Nets {
    nodes: Vec<Option<Node>>, // by ObjId
    links: Vec<Link>,
    pub nodes_serial: i64, // LSD's global nodesSerial counter (0 after loading a configuration)
    /// LSD's turbosearch tree for (parent, label), as the objects in order at INITTURBO time and the size set
    turbo: HashMap<(ObjId, String), (Vec<ObjId>, i64)>,
    /// configuration name and seed (LSD's `simul_name`, `seed - 1`); filled from the command line if unset
    pub sim_name: String,
    pub seed: Option<i64>,
}

impl Default for Nets {
    fn default() -> Self {
        Nets { nodes: Vec::new(), links: Vec::new(), nodes_serial: 0, turbo: HashMap::new(), sim_name: String::new(), seed: None }
    }
}

impl Nets {
    pub fn node(&self, o: ObjId) -> Option<&Node> {
        self.nodes.get(o).and_then(|n| n.as_ref())
    }

    fn node_mut(&mut self, o: ObjId) -> Option<&mut Node> {
        self.nodes.get_mut(o).and_then(|n| n.as_mut())
    }

    pub fn link(&self, l: LinkId) -> &Link {
        &self.links[l]
    }

    pub fn link_mut(&mut self, l: LinkId) -> &mut Link {
        &mut self.links[l]
    }

    /// `new netNode(id, name, prob)`
    fn new_node(&mut self, id: i64, name: &str, prob: f64, t: i32) -> Node {
        self.nodes_serial += 1;
        Node {
            name: if name.is_empty() { None } else { Some(name.to_string()) },
            prob,
            time: t,
            id: if id < 0 { self.nodes_serial } else { id },
            n_links: 0,
            ser_num: self.nodes_serial,
            first: None,
            last: None,
        }
    }

    fn set_node(&mut self, o: ObjId, n: Node) {
        if self.nodes.len() <= o {
            self.nodes.resize(o + 1, None);
        }
        self.nodes[o] = Some(n);
    }

    /// `new netLink(from, to, weight, prob)`: appended at the end of the origin's list; missing nodes are created.
    fn new_link(&mut self, from: ObjId, to: ObjId, weight: f64, prob_to: f64, t: i32) -> LinkId {
        for o in [from, to] {
            if self.node(o).is_none() {
                let n = self.new_node(-1, "", 1.0, t);
                self.set_node(o, n);
            }
        }
        let id = self.links.len();
        let ser_to = self.node(to).unwrap().ser_num;
        let prev = self.node(from).unwrap().last;
        self.links.push(Link { prob_to, weight, time: t, ser_to, next: None, prev, from, to, alive: true });
        let n = self.node(from).unwrap();
        match (n.first, n.last) {
            (Some(_), Some(last)) => self.links[last].next = Some(id),
            _ => self.node_mut(from).unwrap().first = Some(id),
        }
        let n = self.node_mut(from).unwrap();
        n.last = Some(id);
        n.n_links += 1;
        id
    }

    /// `delete link`: unlink keeping the list intact (the dead link keeps its own `next`, as freed memory would).
    fn del_link(&mut self, l: LinkId) {
        let (from, prev, next) = (self.links[l].from, self.links[l].prev, self.links[l].next);
        let (first, last) = {
            let n = self.node(from).unwrap();
            (n.first, n.last)
        };
        if first != Some(l) && last != Some(l) {
            self.links[prev.unwrap()].next = next;
            self.links[next.unwrap()].prev = prev;
        } else if first == Some(l) && last == Some(l) {
            let n = self.node_mut(from).unwrap();
            n.first = None;
            n.last = None;
        } else if first == Some(l) {
            self.node_mut(from).unwrap().first = next;
            self.links[next.unwrap()].prev = None;
        } else {
            self.node_mut(from).unwrap().last = prev;
            self.links[prev.unwrap()].next = None;
        }
        self.node_mut(from).unwrap().n_links -= 1;
        self.links[l].alive = false;
    }

    /// `delete node`: remove all its outgoing links (last first), then the node.
    fn del_node(&mut self, o: ObjId) {
        while let Some(last) = self.node(o).and_then(|n| n.last) {
            self.del_link(last);
        }
        if let Some(slot) = self.nodes.get_mut(o) {
            *slot = None;
        }
    }

    /// The configuration name and seed LSD would have (`simul_name`, `seed - 1`), from the command line
    /// (`lsd run MODEL CONFIG.lsd [--seed N]`) when the engine has not set them.
    pub fn run_info(&mut self) -> (String, i64) {
        if self.sim_name.is_empty() && self.seed.is_none() {
            let a: Vec<String> = std::env::args().collect();
            if a.get(1).map(String::as_str) == Some("run") && a.len() > 3 {
                let p = std::path::Path::new(&a[3]);
                self.sim_name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                self.seed = a.iter().position(|x| x == "--seed").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok());
                if self.seed.is_none() {
                    let text = std::fs::read_to_string(p).unwrap_or_default();
                    self.seed = text.lines().find_map(|l| l.strip_prefix("SEED ")).and_then(|s| s.trim().parse().ok());
                }
            }
        }
        (self.sim_name.clone(), self.seed.unwrap_or(1))
    }
}

/// An object was copied from `ex` (ADDOBJ_EX): LSD gives the copy an isolated node if the first existing
/// instance of its type under the same parent is a node (add_n_objects2: `search(lab)->node != NULL`).
pub fn object_added(m: &mut Model, _ex: ObjId, new: ObjId) {
    let Some(parent) = m.o(new).up else { return };
    let head = m.o(parent).sons.iter().find(|b| *b.label == *m.o(new).label).and_then(|b| b.head);
    if head.map_or(false, |h| m.nets.node(h).is_some()) {
        let t = m.t;
        let n = m.nets.new_node(-1, "", 1.0, t);
        m.nets.set_node(new, n);
    }
}

/// An object is being deleted: LSD deletes its node, which removes the node's outgoing links.
pub fn object_deleted(m: &mut Model, o: ObjId) {
    m.nets.del_node(o);
}

// ------------------------------------------------------------------ helpers on the object tree

fn node(m: &Model, o: ObjId) -> &Node {
    m.nets.node(o).unwrap_or_else(|| fail!("object '{}' has no network data structure", m.label(o)))
}

fn node_mut(m: &mut Model, o: ObjId) -> &mut Node {
    if m.nets.node(o).is_none() {
        fail!("object '{}' has no network data structure", m.label(o));
    }
    m.nets.node_mut(o).unwrap()
}

fn next_of(m: &Model, o: ObjId) -> Option<ObjId> {
    m.o(o).next
}

/// `search_err(lab, ...)`: first instance of `lab` seen from `this`.
fn search_err(m: &Model, this: ObjId, lab: &str) -> Option<ObjId> {
    m.cycle_first(this, lab)
}

/// `check_net_struct`: `this` must be the immediate parent of the objects `lab`; Root cannot be a container.
fn check_net_struct(m: &Model, this: ObjId, lab: &str) -> Option<ObjId> {
    let cur = m.cycle_first(this, lab);
    let Some(cur) = cur else { fail!("object '{}' is missing", lab) };
    let Some(up) = m.o(cur).up else { fail!("cannot create network at the Root level") };
    if *m.o(up).label != *m.o(this).label {
        fail!("no descending object '{}' in container object '{}'", lab, m.label(this));
    }
    Some(cur)
}

/// `nodes2create`: how many more objects are needed (existing ones are kept).
fn nodes2create(m: &Model, this: ObjId, lab: &str, num_nodes: i64) -> i64 {
    let count = m.brothers(search_err(m, this, lab)).len() as i64;
    if num_nodes >= count {
        return num_nodes - count;
    }
    eprint!("\nWarning: number of existing nodes is more than the required.");
    0
}

/// `add_n_objects2(lab, n)`: n copies of the blueprint's object (nothing to do for n = 0).
fn add_n_objects(m: &mut Model, this: ObjId, lab: &str, n: i64) -> Option<ObjId> {
    if n <= 0 {
        return None;
    }
    Some(m.add_objects(this, lab, n as usize, None, -1))
}

/// `for (idNode = 1; cur; cur = go_brother(cur)) cur->add_node_net(idNode++)`; returns the number of nodes.
fn number_nodes(m: &mut Model, first: ObjId) -> i64 {
    let (mut id, mut cur) = (1, Some(first));
    while let Some(c) = cur {
        add_node_net(m, c, id, "", false);
        id += 1;
        cur = next_of(m, c);
    }
    id - 1
}

/// `initturbo(lab, tot)`: remember the objects `lab` of `this` in order; `tot <= 0` means all of them.
fn initturbo(m: &mut Model, this: ObjId, lab: &str, tot: i64) {
    let Some(b) = m.o(this).sons.iter().find(|b| *b.label == *lab) else { fail!("object '{}' is missing for turbo search", lab) };
    let Some(head) = b.head else { fail!("failure when initializing object '{}' for turbo search", lab) };
    let list = m.brothers(Some(head));
    let tot = if tot <= 0 { list.len() as i64 } else { tot };
    m.nets.turbo.insert((this, lab.to_string()), (list, tot));
}

/// `turbosearch(lab, 0, num)`: the num-th object (from 1) as it was at INITTURBO time.
fn turbosearch(m: &Model, this: ObjId, lab: &str, num: i64) -> ObjId {
    let Some((list, tot)) = m.nets.turbo.get(&(this, lab.to_string())) else {
        fail!("object '{}' is not initialized for turbo search", lab)
    };
    let pos = num - 1;
    if pos < 0 || pos >= *tot {
        fail!("position '{}' is invalid for turbo searching object '{}'", num, lab);
    }
    // LSD's tree repeats the last object in slots past the end of the list
    list[(pos as usize).min(list.len() - 1)]
}

// ------------------------------------------------------------------ nodes and links

/// `add_node_net(id, name, silent)`: replaces existing network data, keeping the serial number.
pub fn add_node_net(m: &mut Model, o: ObjId, id: i64, name: &str, silent: bool) -> ObjId {
    let mut ser_old = -1;
    if let Some(n) = m.nets.node(o) {
        if !silent {
            eprint!("\nWarning: existing network data discarded from object.");
        }
        ser_old = n.ser_num;
        m.nets.del_node(o);
    }
    let t = m.t;
    let mut n = m.nets.new_node(id, name, 1.0, t);
    if ser_old > 0 {
        n.ser_num = ser_old;
        m.nets.nodes_serial -= 1;
    }
    m.nets.set_node(o, n);
    o
}

/// `name_node_net`
pub fn name_node_net(m: &mut Model, o: ObjId, name: &str) {
    if let Some(n) = m.nets.node_mut(o) {
        n.name = if name.is_empty() { None } else { Some(name.to_string()) };
    }
}

/// `add_link_net(dest, weight, probTo)`: no check for an existing link; None if parent or type differ.
pub fn add_link_net(m: &mut Model, o: ObjId, dest: ObjId, weight: f64, prob_to: f64) -> Option<LinkId> {
    if m.o(o).up != m.o(dest).up || *m.o(o).label != *m.o(dest).label {
        return None;
    }
    let t = m.t;
    Some(m.nets.new_link(o, dest, weight, prob_to, t))
}

/// `delete_link_net(ptr)`: only if the link belongs to this node.
pub fn delete_link_net(m: &mut Model, o: ObjId, l: Option<LinkId>) {
    let (Some(l), Some(n)) = (l, m.nets.node(o)) else { return };
    let mut cur = n.first;
    while let Some(c) = cur {
        if c == l {
            m.nets.del_link(l);
            return;
        }
        cur = m.nets.links[c].next;
    }
}

/// `search_link_net(destId)`: first link of the node to a node with that id.
pub fn search_link_net(m: &Model, o: ObjId, dest_id: i64) -> Option<LinkId> {
    let n = m.nets.node(o)?;
    let mut cur = n.first;
    while let Some(c) = cur {
        let to = m.nets.links[c].to;
        if node(m, to).id == dest_id {
            break; // a link to an object without node data crashes LSD here, and panics in node()
        }
        cur = m.nets.links[c].next;
    }
    cur
}

/// `draw_link_net`: one outgoing link with probability proportional to probTo.
pub fn draw_link_net(m: &mut Model, o: ObjId) -> Option<LinkId> {
    let first = m.nets.node(o)?.first?;
    let mut sum = 0.0;
    let mut cur = Some(first);
    while let Some(c) = cur {
        let l = &m.nets.links[c];
        if m.nets.node(l.to).is_some() {
            sum += l.prob_to;
        }
        cur = l.next;
    }
    if !sum.is_finite() || sum <= 0.0 {
        fail!("probabilities are invalid for link drawing");
    }
    let draw_point = loop {
        let d = m.rng.ran1() * sum;
        if d != sum {
            break d;
        }
    };
    let (mut acc, mut cur, mut cur1) = (0.0, Some(first), first);
    while acc <= draw_point {
        let Some(c) = cur else { break };
        let l = &m.nets.links[c];
        if m.nets.node(l.to).is_some() {
            acc += l.prob_to;
        }
        cur1 = c;
        cur = l.next;
    }
    Some(cur1)
}

/// `draw_node_net(lab)`: a node with probability proportional to its `prob` (from `this`, the container).
pub fn draw_node_net(m: &mut Model, this: ObjId, lab: &str) -> Option<ObjId> {
    let first = search_err(m, this, lab)?;
    let mut sum = 0.0;
    let mut cur = Some(first);
    while let Some(c) = cur {
        let Some(n) = m.nets.node(c) else { break };
        sum += n.prob;
        cur = next_of(m, c);
    }
    if !sum.is_finite() || sum <= 0.0 {
        fail!("probabilities are invalid for node drawing");
    }
    let draw_point = loop {
        let d = m.rng.ran1() * sum;
        if d != sum {
            break d;
        }
    };
    let (mut acc, mut cur, mut cur2) = (0.0, Some(first), first);
    while acc <= draw_point {
        let Some(c) = cur else { break };
        cur2 = c;
        acc += node(m, c).prob;
        cur = next_of(m, c);
    }
    Some(cur2)
}

/// `draw_rnd(lab)`: an instance with equal probabilities.
fn draw_rnd(m: &mut Model, this: ObjId, lab: &str) -> Option<ObjId> {
    let first = search_err(m, this, lab)?;
    let a = m.brothers(Some(first)).len() as f64;
    let b = loop {
        let b = m.rng.ran1() * a;
        if b != a {
            break b;
        }
    };
    let (mut a, mut cur, mut cur1) = (1.0, first, next_of(m, first));
    while a <= b {
        let Some(c) = cur1 else { break };
        a += 1.0;
        cur = c;
        cur1 = next_of(m, c);
    }
    Some(cur)
}

/// `search_node_net(lab, destId)`: the node object with that id, scanning from the first instance.
pub fn search_node_net(m: &Model, this: ObjId, lab: &str, dest_id: i64) -> Option<ObjId> {
    let mut cur = search_err(m, this, lab);
    while let Some(c) = cur {
        match m.nets.node(c) {
            Some(n) if n.id != dest_id => cur = next_of(m, c),
            _ => break,
        }
    }
    cur.filter(|&c| m.nets.node(c).is_some())
}

/// `stats_net(lab, r)`: r[0] nodes, r[1] arcs, r[2] mean out-degree, r[3] min, r[4] max, r[5] density.
pub fn stats_net(m: &Model, this: ObjId, lab: &str, r: &mut [f64]) -> f64 {
    for x in r.iter_mut().take(6) {
        *x = 0.0;
    }
    let first = search_err(m, this, lab);
    let Some(first) = first.filter(|&f| m.nets.node(f).is_some()) else { return f64::NAN };
    let mut cur = Some(first);
    while let Some(c) = cur {
        if let Some(n) = m.nets.node(c) {
            let n_links = n.n_links as f64;
            if r[0] == 0.0 {
                r[3] = n_links;
            } else {
                r[3] = if r[3] < n_links { r[3] } else { n_links };
            }
            r[0] += 1.0;
            r[1] += n_links;
            r[4] = if r[4] > n_links { r[4] } else { n_links };
        }
        cur = next_of(m, c);
    }
    if r[0] > 0.0 {
        r[2] = r[1] / r[0];
        r[5] = r[1] / (r[0] * (r[0] - 1.0));
    }
    r[0]
}

/// `shuffle_nodes_net(lab)`: Fisher-Yates on the node ids, then sort the objects by id.
pub fn shuffle_nodes_net(m: &mut Model, this: ObjId, lab: &str) -> Option<ObjId> {
    let first = check_net_struct(m, this, lab)?;
    let num_nodes = m.brothers(Some(first)).len() as i64;
    initturbo(m, this, lab, num_nodes);
    for i in (2..=num_nodes).rev() {
        let j = m.rng.uniform_int(1.0, i as f64) as i64;
        let cur = turbosearch(m, this, lab, i);
        let cur1 = turbosearch(m, this, lab, j);
        let (i_id, j_id) = (node(m, cur).id, node(m, cur1).id);
        node_mut(m, cur).id = j_id;
        node_mut(m, cur1).id = i_id;
    }
    // lsdqsort(lab, NULL, "UP"): relink the bridge in id order
    let first = search_err(m, this, lab)?;
    let mut list = m.brothers(Some(first));
    if list.iter().any(|&o| m.nets.node(o).is_none()) {
        fail!("object '{}' has no network data structure", lab);
    }
    list.sort_by_key(|&o| m.nets.node(o).unwrap().id);
    let b = m.o(this).sons.iter().position(|b| *b.label == *lab).unwrap();
    m.objs[this].as_mut().unwrap().sons[b].head = Some(list[0]);
    for w in list.windows(2) {
        m.objs[w[0]].as_mut().unwrap().next = Some(w[1]);
    }
    m.objs[*list.last().unwrap()].as_mut().unwrap().next = None;
    search_err(m, this, lab)
}

/// `delete_net(lab)`: remove the node data of all objects `lab`.
pub fn delete_net(m: &mut Model, this: ObjId, lab: &str) {
    let mut cur = search_err(m, this, lab);
    while let Some(c) = cur {
        m.nets.del_node(c);
        cur = next_of(m, c);
    }
}

// ------------------------------------------------------------------ generators

/// `init_stub_net(lab, gen, numNodes, par1, par2)`: dispatch on the generator name; returns the number of links.
pub fn init_stub_net(m: &mut Model, this: ObjId, lab: &str, gen: &str, mut num_nodes: i64, par1: i64, mut par2: f64) -> f64 {
    let option = gen.to_uppercase();
    if num_nodes <= 0 {
        num_nodes = m.brothers(search_err(m, this, lab)).len() as i64;
    }
    if num_nodes < 2 && option != "DISCONNECTED" {
        fail!("invalid parameter values for a {} network in object '{}'", option, lab);
    }
    match option.as_str() {
        "DISCONNECTED" => return init_discon_net(m, this, lab, num_nodes) as f64,
        "CONNECTED" => return init_connect_net(m, this, lab, num_nodes) as f64,
        "RANDOM-DIR" if par1 > 0 => return init_random_dir_net(m, this, lab, num_nodes, par1) as f64,
        "RANDOM-UNDIR" if par1 > 0 => return init_random_undir_net(m, this, lab, num_nodes, par1) as f64,
        "UNIFORM" if par1 > 0 => return init_uniform_net(m, this, lab, num_nodes, par1) as f64,
        "STAR" => return init_star_net(m, this, lab, num_nodes) as f64,
        "CIRCLE" if par1 > 0 => return init_circle_net(m, this, lab, num_nodes, par1) as f64,
        "RENYI-ERDOS" => {
            if par2 == 0.0 && num_nodes != 0 && par1 != 0 {
                par2 = par1 as f64 / num_nodes as f64;
            }
            if par2 > 0.0 {
                return init_renyi_erdos_net(m, this, lab, num_nodes, par2) as f64;
            }
        }
        "SMALL-WORLD" if par1 > 0 && par2 > 0.0 => return init_small_world_net(m, this, lab, num_nodes, par1, par2) as f64,
        "SCALE-FREE" if par1 > 0 && par2 > 0.0 => return init_scale_free_net(m, this, lab, num_nodes, par1, par2) as f64,
        "LATTICE" if num_nodes % par1 == 0 && par1 > 0 => {
            return init_lattice_net(m, (num_nodes / par1) as i32, par1 as i32, this, lab, (par2 != 0.0) as i32) as f64;
        }
        _ => {}
    }
    fail!("invalid parameter values for a {} network in object '{}'", option, lab)
}

/// Disconnected network: nodes only.
pub fn init_discon_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64) -> i64 {
    if num_nodes < 1 {
        fail!("invalid parameter values for disconnected network in object '{}'", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return -1 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    number_nodes(m, first);
    0
}

/// Fully connected undirected network.
pub fn init_connect_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64) -> i64 {
    if num_nodes < 2 {
        fail!("invalid parameter values for fully connected network in object '{}'", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    number_nodes(m, first);
    let mut links = 0;
    let mut cur1 = search_err(m, this, lab);
    while let Some(c1) = cur1 {
        let mut cur2 = next_of(m, c1);
        while let Some(c2) = cur2 {
            add_link_net(m, c1, c2, 0.0, 1.0);
            add_link_net(m, c2, c1, 0.0, 1.0);
            links += 2;
            cur2 = next_of(m, c2);
        }
        cur1 = next_of(m, c1);
    }
    links
}

/// Star network: the first object is the hub, with two-way links to every other.
pub fn init_star_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64) -> i64 {
    if init_discon_net(m, this, lab, num_nodes) != 0 {
        return 0;
    }
    let Some(hub) = search_err(m, this, lab) else { return 0 };
    let mut links = 0;
    let mut cur2 = next_of(m, hub);
    while let Some(c) = cur2 {
        add_link_net(m, hub, c, 0.0, 1.0);
        add_link_net(m, c, hub, 0.0, 1.0);
        links += 2;
        cur2 = next_of(m, c);
    }
    links
}

/// Random network with a fixed number of directed links.
pub fn init_random_dir_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64, num_links: i64) -> i64 {
    if num_nodes < 2 || num_links < 0 {
        fail!("invalid parameter values for random directed network in object '{}'", lab);
    }
    if num_links > num_nodes * (num_nodes - 1) {
        fail!("object '{}' has numLinks > ( numNodes * ( numNodes - 1 ) )", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    number_nodes(m, first);
    let mut links = 0;
    while links < num_links {
        let cur = draw_node_net(m, this, lab).unwrap();
        let cur1 = draw_node_net(m, this, lab).unwrap();
        if cur != cur1 && search_link_net(m, cur, node(m, cur1).id).is_none() {
            add_link_net(m, cur, cur1, 0.0, 1.0);
            links += 1;
        }
    }
    links
}

/// Random network with a fixed number of directed links, reciprocal (undirected).
pub fn init_random_undir_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64, num_links: i64) -> i64 {
    if num_nodes < 2 || num_links < 0 {
        fail!("invalid parameter values for random undirected network in object '{}'", lab);
    }
    if num_links > (num_nodes * (num_nodes - 1)) / 2 {
        fail!("object '{}' has numLinks > ( numNodes * ( numNodes - 1 ) ) / 2", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    number_nodes(m, first);
    let mut links = 0;
    while links < num_links {
        let cur = draw_node_net(m, this, lab).unwrap();
        let cur1 = draw_node_net(m, this, lab).unwrap();
        if cur != cur1 && search_link_net(m, cur, node(m, cur1).id).is_none() {
            add_link_net(m, cur, cur1, 0.0, 1.0);
            add_link_net(m, cur1, cur, 0.0, 1.0);
            links += 2;
        }
    }
    links
}

/// Uniform random network: a fixed out-degree per node.
pub fn init_uniform_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64, out_deg: i64) -> i64 {
    if num_nodes < 2 || out_deg < 0 || out_deg >= num_nodes {
        fail!("invalid parameter values for uniform random network in object '{}'", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    let num_nodes = number_nodes(m, first);
    initturbo(m, this, lab, num_nodes);
    let mut num_links = 0;
    let mut cur = Some(first);
    while let Some(c) = cur {
        let id_node = node(m, c).id;
        for _ in 1..=out_deg {
            let mut new_node = false;
            let mut try_node = id_node;
            while !new_node || try_node == id_node {
                try_node = m.rng.uniform_int(1.0, num_nodes as f64) as i64;
                new_node = search_link_net(m, c, try_node).is_none();
            }
            let cur1 = turbosearch(m, this, lab, try_node);
            add_link_net(m, c, cur1, 0.0, 1.0);
            num_links += 1;
        }
        cur = next_of(m, c);
    }
    num_links
}

/// Renyi-Erdos: every pair of nodes is linked (both ways) with probability linkProb.
pub fn init_renyi_erdos_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64, link_prob: f64) -> i64 {
    if num_nodes < 2 || link_prob < 0.0 || link_prob > 1.0 {
        fail!("invalid parameter values for Renyi-Erdos network in object '{}'", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    let num_nodes = number_nodes(m, first);
    initturbo(m, this, lab, num_nodes);
    let mut num_links = 0;
    for start in 1..num_nodes {
        for end in start + 1..=num_nodes {
            if m.rng.ran1() < link_prob {
                let cur = turbosearch(m, this, lab, start);
                let cur1 = turbosearch(m, this, lab, end);
                add_link_net(m, cur, cur1, 0.0, 1.0);
                add_link_net(m, cur1, cur, 0.0, 1.0);
                num_links += 2;
            }
        }
    }
    num_links
}

/// Circle: each node linked to outDeg/2 neighbours on each side (by serial number).
pub fn init_circle_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64, out_deg: i64) -> i64 {
    if num_nodes < 2 || out_deg < 0 || out_deg >= num_nodes {
        fail!("invalid parameter values for circle network in object '{}'", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    let num_nodes = number_nodes(m, first);
    initturbo(m, this, lab, num_nodes);
    let mut num_links = 0;
    let mut cur = Some(first);
    while let Some(c) = cur {
        let id_node = node(m, c).ser_num;
        let mut low_neigh = id_node - out_deg / 2;
        for _ in 1..=out_deg {
            while low_neigh == id_node || low_neigh < 1 || low_neigh > num_nodes {
                if low_neigh == id_node {
                    low_neigh += 1;
                }
                if low_neigh < 1 {
                    low_neigh += num_nodes;
                }
                if low_neigh > num_nodes {
                    low_neigh -= num_nodes;
                }
            }
            let cur1 = turbosearch(m, this, lab, low_neigh);
            if search_link_net(m, c, node(m, cur1).id).is_none() {
                add_link_net(m, c, cur1, 0.0, 1.0);
                num_links += 1;
            }
            low_neigh += 1;
        }
        cur = next_of(m, c);
    }
    num_links
}

/// Small-world (Watts and Strogatz 1998): a circle with each forward link rewired with probability rho.
pub fn init_small_world_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64, out_deg: i64, rho: f64) -> i64 {
    if num_nodes < 2 || out_deg < 0 || out_deg >= num_nodes || rho < 0.0 || rho > 1.0 {
        fail!("invalid parameter values for Small-World network in object '{}'", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let num_links = init_circle_net(m, this, lab, num_nodes, out_deg);
    let num_neigh = out_deg / 2;
    let mut cur = Some(first);
    while let Some(c) = cur {
        for link in 1..=num_neigh {
            if m.rng.ran1() < rho {
                let id_node = node(m, c).ser_num;
                let mut try_node = id_node + link;
                if try_node > num_nodes {
                    try_node -= num_nodes;
                }
                let Some(curl) = search_link_net(m, c, try_node) else { continue };
                let cur1 = m.nets.links[curl].to;
                let back = search_link_net(m, cur1, id_node);
                delete_link_net(m, cur1, back);
                let fwd = search_link_net(m, c, try_node);
                delete_link_net(m, c, fwd);
                let mut new_node = id_node;
                while new_node == id_node {
                    new_node = m.rng.uniform_int(1.0, num_nodes as f64) as i64;
                }
                let cur1 = turbosearch(m, this, lab, new_node);
                add_link_net(m, c, cur1, 0.0, 1.0);
                add_link_net(m, cur1, c, 0.0, 1.0);
            }
        }
        cur = next_of(m, c);
    }
    num_links
}

/// Scale-free: preferential attachment in a first scan, then random extra links up to outDeg.
pub fn init_scale_free_net(m: &mut Model, this: ObjId, lab: &str, num_nodes: i64, out_deg: i64, exp_link: f64) -> i64 {
    if num_nodes < 2 || out_deg < 0 || out_deg >= num_nodes || exp_link <= 0.0 {
        fail!("invalid parameter values for scale-free network in object '{}'", lab);
    }
    let Some(first) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    let num_nodes = number_nodes(m, first);
    initturbo(m, this, lab, num_nodes);

    let mut num_links = 0;
    let mut node1 = true;
    let mut cur = Some(first);
    while let Some(mut c) = cur {
        let cur1;
        if node1 {
            node1 = false;
            node_mut(m, c).prob = 1.0; // maximum probability to the first node
            cur1 = c;
            c = next_of(m, c).unwrap();
        } else {
            let cur_prob = node(m, c).prob;
            node_mut(m, c).prob = 0.0; // temporarily remove all probability
            cur1 = draw_node_net(m, this, lab).unwrap();
            node_mut(m, c).prob = cur_prob;
        }
        let n_links = node(m, cur1).n_links + 1;
        node_mut(m, cur1).prob = (n_links as f64).powf(exp_link);
        add_link_net(m, cur1, c, 0.0, 1.0);
        let n_links = node(m, c).n_links + 1;
        node_mut(m, c).prob = (n_links as f64).powf(exp_link);
        add_link_net(m, c, cur1, 0.0, 1.0);
        num_links += 2;
        cur = next_of(m, c);
    }

    let mut cur1 = Some(first);
    while let Some(c1) = cur1 {
        let mut i = 2;
        while i < out_deg {
            let mut l = node(m, c1).first;
            while let Some(lk) = l {
                let to = m.nets.links[lk].to;
                m.nets.links[lk].prob_to = node(m, to).prob;
                node_mut(m, to).prob = 0.0; // remove the destination from draws
                l = m.nets.links[lk].next;
            }
            let cur_prob = node(m, c1).prob;
            node_mut(m, c1).prob = 0.0;
            let cur = draw_rnd(m, this, lab).unwrap(); // equal probabilities, as in LSD
            node_mut(m, c1).prob = cur_prob;
            let mut l = node(m, c1).first;
            while let Some(lk) = l {
                let to = m.nets.links[lk].to;
                node_mut(m, to).prob = m.nets.links[lk].prob_to;
                l = m.nets.links[lk].next;
            }
            add_link_net(m, c1, cur, 0.0, 1.0);
            add_link_net(m, cur, c1, 0.0, 1.0);
            num_links += 2;
            i += 2;
        }
        cur1 = next_of(m, c1);
    }

    // safely remove isolated nodes
    let mut cur = Some(first);
    let mut cur1 = next_of(m, first);
    while let Some(c) = cur {
        if node(m, c).n_links == 0 {
            m.delete(c);
        }
        cur = cur1;
        cur1 = cur1.and_then(|x| next_of(m, x));
    }

    let mut id_node = 1;
    let mut cur = Some(first);
    while let Some(c) = cur {
        node_mut(m, c).id = id_node;
        id_node += 1;
        cur = next_of(m, c);
    }
    num_links
}

/// Lattice on a torus: each cell linked to its 4 (or 8) neighbours, clockwise from North.
pub fn init_lattice_net(m: &mut Model, n_row: i32, n_col: i32, this: ObjId, lab: &str, mut eight: i32) -> i64 {
    let num_nodes = n_row as i64 * n_col as i64;
    eight = if eight == 4 { 0 } else if eight == 8 { 1 } else { eight };
    if n_row <= 0 || n_col <= 0 || (eight != 0 && eight != 1) {
        fail!("invalid parameter values for lattice network in object '{}'", lab);
    }
    let Some(_) = check_net_struct(m, this, lab) else { return 0 };
    let n = nodes2create(m, this, lab, num_nodes);
    add_n_objects(m, this, lab, n);
    let (n_row, n_col) = (n_row as i64, n_col as i64);
    let (mut i, mut j) = (0i64, 0i64);
    let mut cur = m.search(this, lab);
    while let Some(c) = cur {
        add_node_net(m, c, n_col * i + j + 1, "", false);
        j += 1;
        if j >= n_col {
            i += 1;
            j = 0;
        }
        cur = next_of(m, c);
    }
    initturbo(m, this, lab, num_nodes);
    let (mut i, mut j) = (0i64, 0i64);
    let mut num_links = 0;
    let mut cur = m.search(this, lab);
    while let Some(c) = cur {
        let up = if i == 0 { n_row - 1 } else { i - 1 };
        let down = if i == n_row - 1 { 0 } else { i + 1 };
        let right = if j == n_col - 1 { 0 } else { j + 1 };
        let left = if j == 0 { n_col - 1 } else { j - 1 };
        // north, (northeast), east, (southeast), south, (southwest), west, (northwest)
        let mut cells = vec![(up, j), (i, right), (down, j), (i, left)];
        if eight != 0 {
            cells = vec![(up, j), (up, right), (i, right), (down, right), (down, j), (down, left), (i, left), (up, left)];
        }
        for (r, cc) in cells {
            let cur1 = turbosearch(m, this, lab, n_col * r + cc + 1);
            add_link_net(m, c, cur1, 0.0, 1.0);
        }
        num_links += if eight != 0 { 8 } else { 4 };
        j += 1;
        if j >= n_col {
            i += 1;
            j = 0;
        }
        cur = next_of(m, c);
    }
    num_links
}

// ------------------------------------------------------------------ Pajek files

/// C's `%g` (6 significant digits).
fn fmt_g(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let trim = |s: &str| if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s.to_string() };
    let e = format!("{:.5e}", x);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    if exp < -4 || exp >= 6 {
        format!("{}e{}{:02}", trim(mant), if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        trim(&format!("{:.*}", (5 - exp) as usize, x))
    }
}

fn foldersep(dir: &str) -> &'static str {
    if dir.is_empty() { "" } else { "/" }
}

/// The text lines of a file as `fgets` + `feof` would deliver them (a read past the end leaves the buffer unchanged).
struct Reader {
    lines: Vec<String>,
    pos: usize,
    eof: bool,
    buf: String,
}

impl Reader {
    fn fgets(&mut self) {
        if self.pos >= self.lines.len() {
            self.eof = true;
            return;
        }
        self.buf = self.lines[self.pos].clone();
        self.pos += 1;
        if !self.buf.ends_with('\n') {
            self.eof = true; // the last line has no newline: the read hit the end of the file
        }
    }

    /// `get_line`: next line, skipping `%` comments; section lines (`*...`) are upper-cased.
    fn get_line(&mut self) {
        loop {
            self.fgets();
            let first = self.buf.trim_start().chars().next();
            if first != Some('%') || self.eof && self.pos >= self.lines.len() {
                if first == Some('*') {
                    self.buf = self.buf.to_ascii_uppercase();
                }
                return;
            }
        }
    }
}

/// `%ld`: optional sign and digits.
fn scan_long(s: &str) -> Option<(i64, &str)> {
    let s = s.trim_start();
    let b = s.as_bytes();
    let mut i = if matches!(b.first(), Some(b'+') | Some(b'-')) { 1 } else { 0 };
    let d = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == d {
        return None;
    }
    Some((s[..i].parse().ok()?, &s[i..]))
}

/// `%lf`: decimal floating point.
fn scan_double(s: &str) -> Option<f64> {
    let s = s.trim_start();
    let b = s.as_bytes();
    let mut i = if matches!(b.first(), Some(b'+') | Some(b'-')) { 1 } else { 0 };
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut k = i + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        let e0 = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        if k > e0 {
            i = k;
        }
    }
    s[..i].parse().ok()
}

/// `sscanf(line, " %ld %ld %lf", ...)`: the values read before the first failure.
fn scan_arc(line: &str) -> (i64, i64, Option<f64>, usize) {
    let Some((a, rest)) = scan_long(line) else { return (0, 0, None, 0) };
    let Some((b, rest)) = scan_long(rest) else { return (a, 0, None, 1) };
    match scan_double(rest) {
        Some(w) => (a, b, Some(w), 3),
        None => (a, b, None, 2),
    }
}

/// `read_file_net(lab, dir, base, serial, ext)`: load a Pajek network (*Vertices, *Arcs, *Edges) into the
/// objects `lab` of `this`, creating missing ones. Returns the number of links.
pub fn read_file_net(m: &mut Model, this: ObjId, lab: &str, dir: &str, base: &str, serial: i64, ext: &str) -> f64 {
    let mut cur = check_net_struct(m, this, lab);
    if cur.is_none() {
        return 0.0;
    }
    let dot = if ext.is_empty() { "" } else { "." };
    let file_name = if serial >= 0 {
        format!("{}{}{}_{}{}{}", dir, foldersep(dir), base, serial, dot, ext)
    } else {
        format!("{}{}{}{}{}", dir, foldersep(dir), base, dot, ext)
    };
    let Ok(text) = std::fs::read_to_string(&file_name) else { fail!("cannot open network file '{}'", file_name) };
    let mut f = Reader { lines: text.split_inclusive('\n').map(String::from).collect(), pos: 0, eof: false, buf: String::new() };

    let mut num_nodes = 0;
    while !f.eof {
        f.get_line();
        let l = f.buf.trim_start();
        if let Some(rest) = l.strip_prefix("*VERTICES") {
            if let Some((n, _)) = scan_long(rest) {
                num_nodes = n;
                break;
            }
        }
    }
    if num_nodes == 0 {
        fail!("empty or invalid network file '{}'", file_name);
    }

    let mut id_node: i64 = 0; // uninitialised in LSD if the first vertex line cannot be read
    let mut in_section = true;
    let mut count_nodes = 1;
    while count_nodes <= num_nodes {
        let mut name_node = String::new();
        if in_section {
            f.get_line();
            if f.buf.contains('*') {
                in_section = false;
            } else if let Some((id, rest)) = scan_long(&f.buf) {
                id_node = id;
                if let Some(r) = rest.trim_start().strip_prefix('"') {
                    let name = r.find('"').map_or(r, |i| &r[..i]);
                    if !name.is_empty() {
                        name_node = name.to_string();
                    }
                }
            }
        }
        let c = match cur {
            Some(c) => c,
            None => add_n_objects(m, this, lab, 1).unwrap(), // node does not exist yet: create it
        };
        add_node_net(m, c, id_node, &name_node, true);
        count_nodes += 1;
        cur = next_of(m, c);
    }
    let num_nodes = count_nodes - 1;
    if in_section {
        f.get_line();
    }

    let mut num_links = 0;
    initturbo(m, this, lab, num_nodes);
    while !f.eof {
        in_section = true;
        if f.buf.contains("*ARCS") {
            while in_section {
                f.get_line();
                if f.buf.contains('*') || f.eof {
                    in_section = false;
                } else {
                    let (a, b, w, rd) = scan_arc(&f.buf);
                    if rd >= 2 {
                        let from = turbosearch(m, this, lab, a);
                        let to = turbosearch(m, this, lab, b);
                        let l = add_link_net(m, from, to, 0.0, 1.0);
                        if let (Some(w), Some(l)) = (w, l) {
                            m.nets.links[l].weight = w;
                        }
                        num_links += 1;
                    }
                }
            }
        } else if f.buf.contains("*EDGES") {
            while in_section {
                f.get_line();
                if f.buf.contains('*') || f.eof {
                    in_section = false;
                } else {
                    let (a, b, w, rd) = scan_arc(&f.buf);
                    if rd >= 2 {
                        let from = turbosearch(m, this, lab, a);
                        let to = turbosearch(m, this, lab, b);
                        let l2 = add_link_net(m, from, to, 0.0, 1.0);
                        let l3 = add_link_net(m, to, from, 0.0, 1.0);
                        if let Some(w) = w {
                            for l in [l2, l3].into_iter().flatten() {
                                m.nets.links[l].weight = w;
                            }
                        }
                        num_links += 2;
                    }
                }
            }
        } else {
            f.get_line();
        }
    }
    num_links as f64
}

/// `write_file_net(lab, dir, base, serial, append)`: Pajek file `<dir>/<base>_<serial>.net`, or with
/// `append` a `.paj` file with one more network snapshot per call. Returns the number of arcs.
pub fn write_file_net(m: &mut Model, this: ObjId, lab: &str, dir: &str, base: &str, serial: i64, append: bool) -> f64 {
    let t_cur = if m.t > m.max_step { m.max_step } else { m.t };
    let Some(first) = check_net_struct(m, this, lab) else { return 0.0 };
    let ext = if append { "paj" } else { "net" };
    let file_name = if serial >= 0 {
        format!("{}{}{}_{}.{}", dir, foldersep(dir), base, serial, ext)
    } else {
        format!("{}{}{}.{}", dir, foldersep(dir), base, ext)
    };
    let mut out = String::new();
    if append {
        out += &format!("\n*Network {}_{}_{}\n", base, serial, t_cur);
    } else {
        let sim = m.nets.run_info().0;
        out += &format!("% {} objects from LSD '{}' configuration\n", lab, if sim.is_empty() { "(no name)" } else { &sim });
    }
    let nodes = m.brothers(Some(first));
    out += &format!("*Vertices {}\n", nodes.len());
    for &c in &nodes {
        let Some(n) = m.nets.node(c) else { fail!("object '{}' has no network data structure, file '{}' not saved", lab, file_name) };
        match &n.name {
            None => out += &format!("{} \"{}\" [{}-{}]\n", n.ser_num, n.id, n.time, t_cur),
            Some(name) => out += &format!("{} \"{}\" [{}-{}]\n", n.ser_num, name, n.time, t_cur),
        }
    }
    out += "*Arcs\n";
    let mut num_links = 0;
    for &c in &nodes {
        let n = m.nets.node(c).unwrap();
        if n.n_links > 0 {
            let mut l = n.first;
            while let Some(lk) = l {
                let k = &m.nets.links[lk];
                let weight = if k.weight == 0.0 { 1.0 } else { k.weight };
                out += &format!("{} {} {} [{}-{}]\n", n.ser_num, k.ser_to, fmt_g(weight), k.time, t_cur);
                num_links += 1;
                l = k.next;
            }
        }
    }
    let mut opts = std::fs::OpenOptions::new();
    if append && t_cur > 1 {
        opts.append(true).create(true);
    } else {
        opts.write(true).create(true).truncate(true);
    }
    let mut f = opts.open(&file_name).unwrap_or_else(|e| fail!("cannot create network file '{}': {}", file_name, e));
    f.write_all(out.as_bytes()).unwrap();
    num_links as f64
}

// ------------------------------------------------------------------ the macros

impl<'a> Ctx<'a> {
    /// CONFIG: the configuration name being run (LSD's `simul_name`)
    pub fn config(&mut self) -> String {
        self.m.nets.run_info().0
    }

    /// `seed - 1`, the serial number LSD puts in network file names
    pub fn net_serial(&mut self) -> i64 {
        self.m.nets.run_info().1
    }

    // ---- creating and loading networks
    /// INIT_NET / INIT_NETS (O, lab, gen, numNodes, par1, par2); returns the number of links
    pub fn init_net(&mut self, lab: &str, gen: &str, num_nodes: i64, par1: i64, par2: f64) -> f64 {
        let p = self.p;
        init_stub_net(self.m, p, lab, gen, num_nodes, par1, par2)
    }
    pub fn init_nets(&mut self, o: ObjId, lab: &str, gen: &str, num_nodes: i64, par1: i64, par2: f64) -> f64 {
        init_stub_net(self.m, o, lab, gen, num_nodes, par1, par2)
    }
    /// LOAD_NET(lab, base): reads `base_<seed-1>.net` from the current folder
    pub fn load_net(&mut self, lab: &str, base: &str) -> f64 {
        let (p, s) = (self.p, self.net_serial());
        read_file_net(self.m, p, lab, "", base, s, "net")
    }
    pub fn load_nets(&mut self, o: ObjId, lab: &str, base: &str) -> f64 {
        let s = self.net_serial();
        read_file_net(self.m, o, lab, "", base, s, "net")
    }
    /// NETWORK_LOAD(lab, dir, base)
    pub fn network_load(&mut self, lab: &str, dir: &str, base: &str) -> f64 {
        let (p, s) = (self.p, self.net_serial());
        read_file_net(self.m, p, lab, dir, base, s, "net")
    }
    pub fn networks_load(&mut self, o: ObjId, lab: &str, dir: &str, base: &str) -> f64 {
        let s = self.net_serial();
        read_file_net(self.m, o, lab, dir, base, s, "net")
    }
    // ---- saving
    /// SAVE_NET(lab, base): writes `base_<seed-1>.net`
    pub fn save_net(&mut self, lab: &str, base: &str) -> f64 {
        let (p, s) = (self.p, self.net_serial());
        write_file_net(self.m, p, lab, "", base, s, false)
    }
    pub fn save_nets(&mut self, o: ObjId, lab: &str, base: &str) -> f64 {
        let s = self.net_serial();
        write_file_net(self.m, o, lab, "", base, s, false)
    }
    /// SNAP_NET(lab, base): appends a snapshot to `base_<seed-1>.paj`
    pub fn snap_net(&mut self, lab: &str, base: &str) -> f64 {
        let (p, s) = (self.p, self.net_serial());
        write_file_net(self.m, p, lab, "", base, s, true)
    }
    pub fn snap_nets(&mut self, o: ObjId, lab: &str, base: &str) -> f64 {
        let s = self.net_serial();
        write_file_net(self.m, o, lab, "", base, s, true)
    }
    /// NETWORK_SAVE(lab, dir, base)
    pub fn network_save(&mut self, lab: &str, dir: &str, base: &str) -> f64 {
        let (p, s) = (self.p, self.net_serial());
        write_file_net(self.m, p, lab, dir, base, s, false)
    }
    pub fn networks_save(&mut self, o: ObjId, lab: &str, dir: &str, base: &str) -> f64 {
        let s = self.net_serial();
        write_file_net(self.m, o, lab, dir, base, s, false)
    }
    // ---- whole network
    /// STAT_NET / STAT_NETS: fills v[0..6] (nodes, arcs, mean out-degree, min, max, density); returns the node count
    pub fn stat_net(&mut self, lab: &str, v: &mut [f64]) -> f64 {
        stats_net(self.m, self.p, lab, v)
    }
    pub fn stat_nets(&mut self, o: ObjId, lab: &str, v: &mut [f64]) -> f64 {
        stats_net(self.m, o, lab, v)
    }
    pub fn shuffle_net(&mut self, lab: &str) -> Option<ObjId> {
        let p = self.p;
        shuffle_nodes_net(self.m, p, lab)
    }
    pub fn shuffle_nets(&mut self, o: ObjId, lab: &str) -> Option<ObjId> {
        shuffle_nodes_net(self.m, o, lab)
    }
    pub fn delete_net(&mut self, lab: &str) {
        let p = self.p;
        delete_net(self.m, p, lab)
    }
    pub fn delete_nets(&mut self, o: ObjId, lab: &str) {
        delete_net(self.m, o, lab)
    }
    // ---- nodes
    /// V_NODEID / V_NODEIDS
    pub fn v_nodeid(&self) -> f64 {
        node(self.m, self.p).id as f64
    }
    pub fn v_nodeids(&self, o: ObjId) -> f64 {
        node(self.m, o).id as f64
    }
    /// V_NODENAME / V_NODENAMES (None for an unnamed node: LSD returns NULL)
    pub fn v_nodename(&self) -> Option<String> {
        node(self.m, self.p).name.clone()
    }
    pub fn v_nodenames(&self, o: ObjId) -> Option<String> {
        node(self.m, o).name.clone()
    }
    /// WRITE_NODEID / WRITE_NODEIDS (the double is truncated to long)
    pub fn write_nodeid(&mut self, x: f64) -> f64 {
        let p = self.p;
        self.write_nodeids(p, x)
    }
    pub fn write_nodeids(&mut self, o: ObjId, x: f64) -> f64 {
        node_mut(self.m, o).id = x as i64;
        x
    }
    /// WRITE_NODENAME / WRITE_NODENAMES
    pub fn write_nodename(&mut self, name: &str) {
        let p = self.p;
        name_node_net(self.m, p, name)
    }
    pub fn write_nodenames(&mut self, o: ObjId, name: &str) {
        name_node_net(self.m, o, name)
    }
    /// STAT_NODE / STAT_NODES: number of outgoing links
    pub fn stat_node(&self) -> f64 {
        node(self.m, self.p).n_links as f64
    }
    pub fn stat_nodes(&self, o: ObjId) -> f64 {
        node(self.m, o).n_links as f64
    }
    /// SEARCH_NODE / SEARCH_NODES (O, lab, id)
    pub fn search_node(&self, lab: &str, id: i64) -> Option<ObjId> {
        search_node_net(self.m, self.p, lab, id)
    }
    pub fn search_nodes(&self, o: ObjId, lab: &str, id: i64) -> Option<ObjId> {
        search_node_net(self.m, o, lab, id)
    }
    /// RNDDRAW_NODE / RNDDRAW_NODES
    pub fn rnddraw_node(&mut self, lab: &str) -> Option<ObjId> {
        let p = self.p;
        draw_node_net(self.m, p, lab)
    }
    pub fn rnddraw_nodes(&mut self, o: ObjId, lab: &str) -> Option<ObjId> {
        draw_node_net(self.m, o, lab)
    }
    /// DRAWPROB_NODE / DRAWPROB_NODES
    pub fn drawprob_node(&mut self, x: f64) -> f64 {
        let p = self.p;
        self.drawprob_nodes(p, x)
    }
    pub fn drawprob_nodes(&mut self, o: ObjId, x: f64) -> f64 {
        node_mut(self.m, o).prob = x;
        x
    }
    /// ADDNODE / ADDNODES (id, name): make the object a node, discarding existing network data
    pub fn add_node(&mut self, id: i64, name: &str) -> ObjId {
        let p = self.p;
        add_node_net(self.m, p, id, name, false)
    }
    pub fn add_nodes(&mut self, o: ObjId, id: i64, name: &str) -> ObjId {
        add_node_net(self.m, o, id, name, false)
    }
    /// DELETE_NODE / DELETE_NODES: remove the network data (and outgoing links), keep the object
    pub fn delete_node(&mut self) {
        let p = self.p;
        self.m.nets.del_node(p)
    }
    pub fn delete_nodes(&mut self, o: ObjId) {
        self.m.nets.del_node(o)
    }
    // ---- links
    /// ADDLINK / ADDLINKW / ADDLINKS / ADDLINKWS: new link at the end of the origin's list (None if not siblings)
    pub fn add_link(&mut self, dest: ObjId) -> Option<LinkId> {
        let p = self.p;
        add_link_net(self.m, p, dest, 0.0, 1.0)
    }
    pub fn add_linkw(&mut self, dest: ObjId, w: f64) -> Option<LinkId> {
        let p = self.p;
        add_link_net(self.m, p, dest, w, 1.0)
    }
    pub fn add_links(&mut self, o: ObjId, dest: ObjId) -> Option<LinkId> {
        add_link_net(self.m, o, dest, 0.0, 1.0)
    }
    pub fn add_linkws(&mut self, o: ObjId, dest: ObjId, w: f64) -> Option<LinkId> {
        add_link_net(self.m, o, dest, w, 1.0)
    }
    /// DELETE_LINK
    pub fn delete_link(&mut self, l: LinkId) {
        let from = self.m.nets.links[l].from;
        delete_link_net(self.m, from, Some(l))
    }
    /// SEARCH_LINK / SEARCH_LINKS (O, id): first link to the node with this id
    pub fn search_link(&self, id: i64) -> Option<LinkId> {
        search_link_net(self.m, self.p, id)
    }
    pub fn search_links(&self, o: ObjId, id: i64) -> Option<LinkId> {
        search_link_net(self.m, o, id)
    }
    /// RNDDRAW_LINK / RNDDRAW_LINKS
    pub fn rnddraw_link(&mut self) -> Option<LinkId> {
        let p = self.p;
        draw_link_net(self.m, p)
    }
    pub fn rnddraw_links(&mut self, o: ObjId) -> Option<LinkId> {
        draw_link_net(self.m, o)
    }
    /// DRAWPROB_LINK
    pub fn drawprob_link(&mut self, l: LinkId, x: f64) -> f64 {
        self.m.nets.links[l].prob_to = x;
        x
    }
    /// LINKTO / LINKFROM
    pub fn linkto(&self, l: LinkId) -> ObjId {
        self.m.nets.links[l].to
    }
    pub fn linkfrom(&self, l: LinkId) -> ObjId {
        self.m.nets.links[l].from
    }
    /// V_LINK: weight; WRITE_LINK
    pub fn v_link(&self, l: LinkId) -> f64 {
        self.m.nets.links[l].weight
    }
    pub fn write_link(&mut self, l: LinkId, x: f64) -> f64 {
        self.m.nets.links[l].weight = x;
        x
    }
    /// CYCLE_LINK / CYCLE_LINKS, live like LSD's `for (X = node->first; X; X = X->next)`:
    /// `let mut l = c.first_link(); while let Some(x) = l { ...; l = c.next_link(x); }`
    pub fn first_link(&self) -> Option<LinkId> {
        node(self.m, self.p).first
    }
    pub fn first_links(&self, o: ObjId) -> Option<LinkId> {
        node(self.m, o).first
    }
    pub fn next_link(&self, l: LinkId) -> Option<LinkId> {
        self.m.nets.links[l].next
    }
    /// The links of the node now, as a snapshot (safe to modify the node while looping; LSD's live loop
    /// would also visit links added meanwhile and misbehave on deleted ones).
    pub fn cycle_link(&self) -> Vec<LinkId> {
        self.cycle_links(self.p)
    }
    pub fn cycle_links(&self, o: ObjId) -> Vec<LinkId> {
        let mut v = Vec::new();
        let mut l = self.first_links(o);
        while let Some(x) = l {
            v.push(x);
            l = self.m.nets.links[x].next;
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_format_matches_printf() {
        // values checked against C printf("%g")
        for (x, s) in [(9.9, "9.9"), (-0.1, "-0.1"), (1.0, "1"), (2.5, "2.5"), (1e6, "1e+06"), (123456.7, "123457"),
                       (0.0001, "0.0001"), (0.00001, "1e-05"), (18.9, "18.9"), (6.7, "6.7"), (999999.5, "1e+06"), (0.0, "0")] {
            assert_eq!(fmt_g(x), s);
        }
    }

    #[test]
    fn pajek_scanning() {
        assert_eq!(scan_arc("  1 3 2.5\n"), (1, 3, Some(2.5), 3));
        assert_eq!(scan_arc("4 5\n"), (4, 5, None, 2));
        assert_eq!(scan_arc("7\n").3, 1);
        assert_eq!(scan_arc("x 1\n").3, 0);
        assert_eq!(scan_arc("1 2 3e2 [4-5]").2, Some(300.0));
    }
}
