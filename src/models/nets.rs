//! Example/Test/Nets (fun_nets.cpp): network generation and manipulation.
use crate::model::{Ctx, EqDef, Model};

/// Initialize the net (typeNet selects the generator).
fn init_net(c: &mut Ctx) -> f64 {
    init_net_with(c, false)
}

/// `gen`: also offer DISCONNECTED, CONNECTED and RENYI-ERDOS by degree as types 10, 11 and 12 (NetsGen only).
fn init_net_with(c: &mut Ctx, gen: bool) -> f64 {
    let mut v = [0.0f64; 10];
    let cur = c.search("Network").expect("Network"); // network nodes container (parent object)
    v[0] = c.v("typeNet");
    v[1] = c.v("numNodes");
    v[2] = c.v("numLinks");
    v[3] = c.v("avgOutDeg");
    v[4] = c.v("linkProb");
    v[5] = c.v("rho");
    v[6] = c.v("expLink");
    v[7] = c.v("nCol");
    v[8] = c.v("eightNeigbr");

    match v[0] as i32 {
        0 => v[2] = c.load_nets(cur, "Node", "net"),
        1 => v[2] = c.init_nets(cur, "Node", "RANDOM-DIR", v[1] as i64, v[2] as i64, 0.0),
        2 => v[2] = c.init_nets(cur, "Node", "RANDOM-UNDIR", v[1] as i64, v[2] as i64, 0.0),
        3 => v[2] = c.init_nets(cur, "Node", "UNIFORM", v[1] as i64, v[3] as i64, 0.0),
        4 => v[2] = c.init_nets(cur, "Node", "RENYI-ERDOS", v[1] as i64, 0, v[4]),
        5 => v[2] = c.init_nets(cur, "Node", "CIRCLE", v[1] as i64, v[3] as i64, 0.0),
        6 => v[2] = c.init_nets(cur, "Node", "SMALL-WORLD", v[1] as i64, v[3] as i64, v[5]),
        7 => v[2] = c.init_nets(cur, "Node", "SCALE-FREE", v[1] as i64, v[3] as i64, v[6]),
        8 => v[2] = c.init_nets(cur, "Node", "STAR", v[1] as i64, 0, 0.0),
        9 => v[2] = c.init_nets(cur, "Node", "LATTICE", v[1] as i64, v[7] as i64, v[8]),
        10 if gen => v[2] = c.init_nets(cur, "Node", "DISCONNECTED", v[1] as i64, 0, 0.0),
        12 if gen => v[2] = c.init_nets(cur, "Node", "RENYI-ERDOS", v[1] as i64, v[3] as i64, 0.0),
        11 if gen => v[2] = c.init_nets(cur, "Node", "CONNECTED", v[1] as i64, 0, 0.0),
        _ => {} // invalid type of network
    }

    c.stat_nets(cur, "Node", &mut v); // get network statistics

    c.writes(cur, "numNodes", v[0]); // updates parameters
    c.writes(cur, "numLinks", v[1]);
    c.writes(cur, "avgOutDeg", v[2]);

    c.shuffle_nets(cur, "Node"); // shuffles node sequence (IDs and linked list order)

    c.parameter(); // run variable just once
    1.0
}

/// Add one new bidirectional link randomly every time step, with weight equal to the time.
fn new_link(c: &mut Ctx) -> f64 {
    let cur1 = c.rnddraw_node("Node").expect("node"); // draw first node
    let cur2 = c.rnddraw_node("Node").expect("node"); // draw second node
    let t = c.t();
    c.add_linkws(cur1, cur2, t); // link from 1st to 2nd
    c.add_linkws(cur2, cur1, t); // link from 2nd to 1st
    let name = c.config();
    c.snap_net("Node", &name); // create a network snapshot
    1.0
}

/// Register the total number of links in network.
fn total_links(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 10];
    c.stat_net("Node", &mut v);
    v[1]
}

/// Register the total number of links in the node.
/// The C++ has `STAT_NODE; RESULT(v[0])`: it discards the count and returns an uninitialised stack
/// value (different at every run), so no port can match it. Here: the evident intent, the count.
fn node_links(c: &mut Ctx) -> f64 {
    c.stat_node()
}

/// Register the id of the highest weight link destination for the node and reduce the weight by one.
fn top_link(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 10];
    v[0] = 0.0; // max weight
    v[1] = 0.0; // max weight link dest. id
    let mut cur = None; // pointer to destination
    let mut curl1 = None;

    let mut curl = c.first_link();
    while let Some(l) = curl {
        if c.v_link(l) > v[0] {
            // is this link bigger?
            v[0] = c.v_link(l); // save the new top weight
            let to = c.linkto(l); // to where it points
            cur = Some(to);
            curl1 = Some(l);
            v[1] = c.v_nodeids(to); // and its id
        }
        curl = c.next_link(l);
    }

    if let Some(l) = curl1 {
        c.write_link(l, v[0] - 0.1); // reduces weight of top link
    }

    if v[1] != 0.0 {
        // search top pointed node
        let parent = c.parent();
        let cur2 = c.search_nodes(parent, "Node", v[1] as i64);
        if cur.is_some() && cur != cur2 {
            // check pointer & id match
            c.log("\nWarning: pointer mismatch");
        } else {
            let cur = cur.unwrap();
            v[2] = c.v_nodeids(cur);
            v[3] = c.v("numNodes");
            c.write_nodeids(cur, v[2] + v[3]); // change top node id
        }
    }
    v[1]
}

/// Order-sensitive signature of the node: its id and the sequence of link destination ids and weights (NetsGen).
fn sig(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 10];
    v[0] = c.v_nodeid() * 1000003.0;
    v[1] = 0.0;
    let mut curl = c.first_link();
    while let Some(l) = curl {
        v[1] += 1.0;
        v[0] += v[1] * c.v_nodeids(c.linkto(l)) + c.v_link(l) * 0.37;
        curl = c.next_link(l);
    }
    v[2] = c.stat_node();
    v[0] + v[2] / 7.0
}

// ------------------------------------------------------------------ NetsOps (tests/lsd-models/NetsOps)

/// Write the Pajek file loaded by region 2.
fn ops_setup(c: &mut Ctx) -> f64 {
    let name = format!("netsops_in_{}.net", c.net_serial());
    let text = "% input network for NetsOps\n*Vertices 8\n\
        1 \"alpha\" 0.1 0.2\n  2 \"beta\"\n3 \"gamma delta\"\n4\n\
        % a comment between vertices\n5 \"e\"\n6 \"f\"\n7 \"g\"\n8 \"h\"\n\
        *arcs\n1 2\n1 3 2.5\n2 3 0.5\n3 1\n% another comment\n4 5 7\n\
        *Edges\n5 6 1.5\n6 7\n7 8 3\n\
        *ARCS\n8 1 9\n1 8 0.25\n";
    std::fs::write(name, text).unwrap();
    c.parameter();
    1.0
}

/// Statistics of the network of every region.
fn ops_report(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 10];
    v[9] = 0.0;
    let mut cur = c.first("Region");
    while let Some(r) = cur {
        v[8] = c.stat_nets(r, "Agent", &mut v); // NaN if the objects are not nodes
        if v[8] != v[8] {
            v[9] += 1e6;
        } else {
            v[9] += v[0] + 10.0 * v[1] + 100.0 * v[2] + 1000.0 * v[3] + 10000.0 * v[4] + 100000.0 * v[5];
        }
        cur = c.next(r);
    }
    if c.m.t == 9 {
        // last step: operations on the networks from outside
        let mut cur = c.first("Region");
        while let Some(r) = cur {
            let h = c.vs(r, "regionId") as i32;
            let nm = format!("netsops_s{}", h);
            v[9] += c.save_nets(r, "Agent", &nm);
            v[9] += c.snap_nets(r, "Agent", &nm);

            let cur1 = c.rnddraw_nodes(r, "Agent").unwrap();
            let curl = c.rnddraw_links(cur1);
            v[9] += match curl { None => 1.0, Some(l) => c.v_link(l) };
            c.write_nodenames(cur1, "last");
            v[9] += c.v_nodenames(cur1).unwrap().len() as f64;

            if h == 2 {
                // load the file again over the existing nodes (one is missing)
                v[9] += c.load_nets(r, "Agent", "netsops_in");
            }
            if h == 3 {
                // delete the network and create another on the same objects
                c.delete_nets(r, "Agent");
                v[8] = c.stat_nets(r, "Agent", &mut v);
                v[9] += if v[8] != v[8] { 11.0 } else { 1.0 };
                v[9] += c.init_nets(r, "Agent", "UNIFORM", 0, 3, 0.0);
            }
            cur = c.next(r);
        }
    }
    v[9]
}

/// Create the network of the region.
fn ops_build(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 10];
    v[0] = c.v("regionId");
    if v[0] == 1.0 {
        v[1] = c.init_net("Agent", "SMALL-WORLD", 20, 4, 0.3);
    } else if v[0] == 2.0 {
        v[1] = c.network_load("Agent", "", "netsops_in");
    } else {
        v[1] = c.init_net("Agent", "CIRCLE", 12, 4, 0.0);
    }
    c.stat_net("Agent", &mut v);
    c.parameter();
    v[1] + 1000.0 * v[0]
}

/// Operations on the network of the region, depending on time and region.
fn ops_ops(c: &mut Ctx) -> f64 {
    let mut v = [0.0f64; 10];
    let h = c.v("regionId") as i32;
    v[9] = 0.0;
    let t = c.m.t;
    match t {
        2 => {
            // draws and searches
            for _ in 0..4 {
                let cur = c.rnddraw_node("Agent").unwrap();
                v[9] += c.v_nodeids(cur) + c.stat_nodes(cur);
            }
            let cur = c.search_node("Agent", 5);
            v[9] += match cur { None => 1000.0, Some(x) => c.v_nodeids(x) * 7.0 };
            let cur = c.search_node("Agent", 9999); // does not exist
            v[9] += if cur.is_none() { 5.0 } else { 0.0 };
        }
        3 if h != 4 => {
            // new objects in a network get isolated nodes
            let cur = c.rnddraw_node("Agent").unwrap();
            let cur1 = c.addobj_ex("Agent", cur); // copy of an example
            let cur2 = c.addobj("Agent"); // copy of the blueprint
            let cur3 = c.rnddraw_node("Agent").unwrap();
            v[9] += c.stat_nodes(cur1) + c.stat_nodes(cur2) + c.v_nodeids(cur1) + c.v_nodeids(cur2) * 3.0;
            c.add_links(cur1, cur3);
            c.add_linkws(cur2, cur1, 4.5);
            c.add_links(cur3, cur2);
            if h == 1 {
                // copy the region: its agents will not be nodes
                let parent = c.parent();
                let p = c.p;
                let cur4 = c.addobjs_ex(parent, "Region", p);
                c.writes(cur4, "regionId", 4.0);
            }
        }
        4 if h == 4 => {
            // the copied region
            v[1] = c.stat_net("Agent", &mut v);
            v[9] += if v[1] != v[1] { 777.0 } else { 1.0 };
            v[2] = c.init_net("Agent", "STAR", 0, 0, 0.0); // all existing objects become nodes
            c.stat_net("Agent", &mut v);
            v[9] += v[0] + 3.0 * v[1] + 5.0 * v[2];
        }
        5 => {
            let nm = format!("netsops_r{}", h);
            if h == 3 {
                // remove the network and rebuild it (fewer nodes than objects)
                c.delete_net("Agent");
                v[1] = c.stat_net("Agent", &mut v);
                v[9] += if v[1] != v[1] { 888.0 } else { 1.0 };
                v[2] = c.init_net("Agent", "RANDOM-UNDIR", 10, 14, 0.0);
                c.stat_net("Agent", &mut v);
                v[9] += 10.0 * v[0] + v[2];
            } else {
                c.shuffle_net("Agent");
            }
            v[9] += c.save_net("Agent", &nm);
        }
        6 if h == 2 || h == 1 => {
            let cur = c.search_node("Agent", if h == 2 { 3 } else { 5 });
            v[9] += if cur.is_none() { 4000.0 } else { 1.0 };
            if let Some(cur) = cur {
                // remove the links to the node first
                let mut a = c.first("Agent");
                while let Some(cur1) = a {
                    let mut curl = c.first_links(cur1);
                    while let Some(l) = curl {
                        let curl1 = c.next_link(l);
                        if c.linkto(l) == cur {
                            c.delete_link(l);
                        }
                        curl = curl1;
                    }
                    a = c.next(cur1);
                }
                if h == 2 {
                    c.delete(cur); // delete the object, node and links
                } else {
                    c.delete_nodes(cur); // delete only the node (network data)
                }
                c.stat_net("Agent", &mut v);
                v[9] += v[0] + v[1];
            }
        }
        7 => {
            if h == 1 {
                // find the object without network data and make it a node again
                let cur = c.search_node("Agent", 5);
                v[9] += if cur.is_none() { 5000.0 } else { 1.0 };
                let mut cur = c.first("Agent");
                while let Some(x) = cur {
                    if c.m.nets.node(x).is_none() {
                        break;
                    }
                    cur = c.next(x);
                }
                if let Some(x) = cur {
                    c.add_nodes(x, 99, "revived");
                }
            }
            let nm = format!("netsops_r{}", h);
            v[9] += c.snap_net("Agent", &nm);
        }
        8 => {
            let nm = format!("netsops_r{}", h);
            v[9] += c.snap_net("Agent", &nm);
            if h == 4 {
                // delete the last three agents of the copied region
                for _ in 0..3 {
                    let mut last = None; // find the last one
                    let mut cur = c.first("Agent");
                    while let Some(x) = cur {
                        last = Some(x);
                        cur = c.next(x);
                    }
                    let cur1 = last.unwrap();
                    // remove the links to it first
                    let mut cur = c.first("Agent");
                    while let Some(x) = cur {
                        let mut curl = c.first_links(x);
                        while let Some(l) = curl {
                            let curl1 = c.next_link(l);
                            if c.linkto(l) == cur1 {
                                c.delete_link(l);
                            }
                            curl = curl1;
                        }
                        cur = c.next(x);
                    }
                    c.delete(cur1);
                    c.stat_net("Agent", &mut v);
                    v[9] += v[0] * 10.0 + v[1];
                }
            }
        }
        _ => {}
    }
    v[9]
}

/// Operations of a node on its links and on other nodes.
fn ops_ag_ops(c: &mut Ctx) -> f64 {
    if c.m.nets.node(c.p).is_none() {
        return -1.0; // not a node
    }
    let mut v = [0.0f64; 10];
    let (p, t) = (c.p, c.m.t);
    v[9] = 0.0;
    match t {
        2 => {
            // new links and drawing probabilities
            let parent = c.parent();
            let cur = c.rnddraw_nodes(parent, "Agent").unwrap();
            if cur != p && c.search_link(c.v_nodeids(cur) as i64).is_none() {
                let curl = c.add_linkw(cur, t as f64 + 0.5).unwrap();
                let curl1 = c.add_linkws(cur, p, 2.0 * t as f64).unwrap();
                c.drawprob_link(curl, 3.0);
                v[9] += c.v_link(curl) + c.v_link(curl1) + c.stat_nodes(cur);
            }
            let r = c.rnd();
            c.drawprob_node(1.0 + 2.0 * r);
        }
        3 => {
            // draw, change and delete links
            if let Some(curl) = c.rnddraw_link() {
                v[9] += c.v_link(curl);
                let w = c.v_link(curl) * 2.0;
                c.write_link(curl, w);
                let cur = c.linkto(curl);
                let id = c.v_nodeid() as i64;
                if let Some(curl2) = c.search_links(cur, id) {
                    v[9] += 100.0 + c.v_link(curl2) + if c.linkfrom(curl2) == cur { 1000.0 } else { 0.0 };
                }
                if (id as i32) % 5 == 0 {
                    c.delete_link(curl);
                }
            }
        }
        4 => {
            // names and ids, replacing the node data
            let id = c.v_nodeid() as i64;
            if (id as i32) % 2 == 0 {
                c.write_nodeid((id + 1000) as f64);
            }
            let id = c.v_nodeid() as i64;
            let nm = format!("ag{}_{}", id as i32, t);
            c.write_nodename(&nm);
            v[9] += c.v_nodename().unwrap().len() as f64;
            if (id as i32) % 7 == 0 {
                c.add_node(id, "reborn"); // discards the links, keeps the serial number
                v[9] += c.stat_node() + c.v_nodename().unwrap().len() as f64;
            }
        }
        5 => {
            // links to nodes of this and of another region
            let parent = c.parent();
            let cur = c.rnddraw_nodes(parent, "Agent").unwrap();
            let pr = ((c.v_nodeids(cur) as i64 as i32) % 3 + 1) as f64;
            c.drawprob_nodes(cur, pr);
            let curl = c.add_links(p, cur);
            v[9] += match curl { None => 1.0, Some(l) => c.v_link(l) + 2.0 };
            if let Some(next_region) = c.next(parent) {
                let cur2 = c.search_nodes(next_region, "Agent", 1);
                let curl2 = cur2.and_then(|x| c.add_links(p, x));
                v[9] += if curl2.is_none() { 77.0 } else { 88.0 };
            }
        }
        7 => {
            // unweighted links
            let parent = c.parent();
            let cur = c.rnddraw_nodes(parent, "Agent").unwrap();
            if cur != p {
                let curl = c.add_link(cur).unwrap();
                v[9] += c.v_link(curl) + 1.0;
            }
        }
        9 => {
            // delete the node data of some agents, after the links to them
            if (c.v_nodeid() as i64 as i32) % 3 == 0 {
                let parent = c.parent();
                let mut cur = c.firsts(parent, "Agent");
                while let Some(x) = cur {
                    if c.m.nets.node(x).is_some() {
                        let mut curl = c.first_links(x);
                        while let Some(l) = curl {
                            let curl1 = c.next_link(l);
                            if c.linkto(l) == p {
                                c.delete_link(l);
                            }
                            curl = curl1;
                        }
                    }
                    cur = c.next(x);
                }
                v[9] += c.stat_node();
                c.delete_node();
            }
        }
        6 => {
            // cycle over the links of a neighbour
            if let Some(curl) = c.first_link() {
                let cur = c.linkto(curl);
                let mut l = c.first_links(cur);
                while let Some(x) = l {
                    let to = c.linkto(x);
                    v[9] += c.v_link(x) + c.v_nodeids(to);
                    l = c.next_link(x);
                }
            }
        }
        _ => {}
    }
    v[9]
}

/// Order-sensitive signature of the node: its id, name and the sequence of link destination ids and weights.
fn ops_ag_sig(c: &mut Ctx) -> f64 {
    if c.m.nets.node(c.p).is_none() {
        return -1.0;
    }
    let mut v = [0.0f64; 10];
    v[0] = (c.v_nodeid() as i64 * 1000003) as f64;
    v[1] = 0.0;
    let mut curl = c.first_link();
    while let Some(l) = curl {
        v[1] += 1.0;
        let to = c.linkto(l);
        v[0] += v[1] * c.v_nodeids(to) + c.v_link(l) * 0.37;
        curl = c.next_link(l);
    }
    if let Some(name) = c.v_nodename() {
        v[0] += (name.len() * 31) as f64;
    }
    v[0] + c.stat_node() / 7.0
}

pub fn equations() -> Vec<(&'static str, EqDef)> {
    vec![
        ("InitNet", EqDef::Fn(init_net)),
        ("NewLink", EqDef::Fn(new_link)),
        ("TotalLinks", EqDef::Fn(total_links)),
        ("NodeLinks", EqDef::Fn(node_links)),
        ("TopLink", EqDef::Fn(top_link)),
        ("close_sim", EqDef::Hook(close_sim)),
    ]
}

fn init_net_gen(c: &mut Ctx) -> f64 {
    init_net_with(c, true)
}

/// Models of our own, checked against LSD with `check.sh tests/lsd-models/<Name>`; registered by `get`.
pub fn get(name: &str) -> Option<Vec<(&'static str, EqDef)>> {
    match name {
        // Example/Test/Nets with `v[0] = STAT_NODE` in NodeLinks (see node_links), same equations
        "NetsFixed" => Some(equations()),
        // the same, with every generator selectable (typeNet 1 to 12)
        "NetsGen" => {
            let mut e = equations();
            e[0] = ("InitNet", EqDef::Fn(init_net_gen));
            e.push(("Sig", EqDef::Fn(sig)));
            Some(e)
        }
        "NetsOps" => Some(vec![
            ("Setup", EqDef::Fn(ops_setup)),
            ("Report", EqDef::Fn(ops_report)),
            ("Build", EqDef::Fn(ops_build)),
            ("Ops", EqDef::Fn(ops_ops)),
            ("AgOps", EqDef::Fn(ops_ag_ops)),
            ("AgSig", EqDef::Fn(ops_ag_sig)),
        ]),
        _ => None,
    }
}

/// close_sim(): save the final network as a Pajek file; the engine calls it after the run.
pub fn close_sim(m: &mut Model) {
    let root = m.root;
    let network = m.search(root, "Network").expect("Network");
    let (name, serial) = m.nets.run_info();
    crate::nets::write_file_net(m, network, "Node", "", &name, serial, false);
}
