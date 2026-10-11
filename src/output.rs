//! Results as LSD writes them with `-t` (CSV): saved series of live objects in tree order,
//! then those of deleted objects (the cemetery); "NA" where a series has no value.
use crate::model::{fmt17_into, Model, ObjId};

pub fn csv(m: &Model) -> String {
    struct Col<'a> { name: String, data: &'a [f64], start: i32, end: i32 }
    let mut cols: Vec<Col> = Vec::new();
    fn walk<'a>(m: &'a Model, o: ObjId, cols: &mut Vec<Col<'a>>) {
        let obj = m.o(o);
        for v in obj.vars.iter().filter(|v| v.save) {
            let lab_tit = m.lab_tit(o);
            // LSD prints no suffix for the only instance of a top-level chain
            let single = matches!(lab_tit.as_str(), "1" | "1_1" | "1_1_1" | "1_1_1_1") && m.hyper_next(o).is_none();
            let name = if single { v.label.to_string() } else { format!("{}_{}", v.label, lab_tit) };
            cols.push(Col { name, data: &v.data, start: v.start, end: v.end });
        }
        for b in &obj.sons {
            if let Some(h) = b.head {
                if m.o(h).to_compute {
                    for c in m.brothers(Some(h)) {
                        walk(m, c, cols);
                    }
                }
            }
        }
    }
    walk(m, m.root, &mut cols);
    for d in &m.cemetery {
        cols.push(Col { name: format!("{}_{}", d.label, d.lab_tit), data: &d.data, start: d.start, end: d.end });
    }
    let mut s = cols.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(",");
    s.push('\n');
    for t in 1..=m.steps_done {
        for (i, c) in cols.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let x = if c.start <= t && c.end >= t { c.data.get((t - c.start) as usize).copied() } else { None };
            match x {
                Some(x) if !x.is_nan() => fmt17_into(&mut s, x),
                _ => s.push_str("NA"),
            }
        }
        s.push('\n');
    }
    s
}
