//! Reader for LSD 8.1 configuration files (`.lsd`): model structure, initial
//! values and simulation settings. Mirrors `load_configuration`, `load_struct`
//! and `load_param` of LSD's file.cpp; descriptions and the embedded equation
//! file that follow MODELREPORT are ignored.
//!
//! Instance order: instances of an object type are listed in tree order, i.e.
//! grouped by parent instance (parents themselves in tree order). `counts` has
//! one entry per instance of the parent type, so `counts.len()` equals the
//! parent's `total()`, and instance `k` of the type has values `values[k]`.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Var,
    Param,
    Func,
}

#[derive(Debug, Clone)]
pub struct ElemSpec {
    pub label: String,
    pub kind: Kind,
    pub num_lag: usize,
    pub save: bool,
    pub savei: bool,
    pub debug: char,
    pub plot: bool,
    pub parallel: bool,
    /// false when the file marks the values as not initialized ('-'); they are then 0 placeholders
    pub initialized: bool,
    pub upd: Option<[i32; 4]>, // delay, delay_range, period, period_range
    pub values: Vec<Vec<f64>>, // one entry per instance (tree order); len 1 for Param, num_lag for Var/Func
}

#[derive(Debug, Clone)]
pub struct ObjSpec {
    pub label: String,
    pub to_compute: bool,
    pub counts: Vec<usize>, // instances per parent instance, in tree order (Root: [1])
    pub elems: Vec<ElemSpec>, // structure order
    pub sons: Vec<ObjSpec>, // structure order
}

impl ObjSpec {
    /// Total number of instances of this object type in the whole model.
    pub fn total(&self) -> usize {
        self.counts.iter().sum()
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub root: ObjSpec,
    pub sim_num: u32,
    pub seed: u32,
    pub max_step: u32,
    /// the up to 7 optional ints after max_step (when_debug, stack_info, prof_min_msecs, ...)
    pub max_step_extra: Vec<i32>,
    pub equation_file: String,
    pub model_report: String,
}

pub fn load(path: &Path) -> Result<Config, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    parse(&String::from_utf8_lossy(&bytes)).map_err(|e| format!("{}: {}", path.display(), e))
}

/// Token scanner with fscanf-like semantics (whitespace-insensitive).
struct Scan<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Scan<'a> {
    fn skip_ws(&mut self) {
        let b = self.s.as_bytes();
        while self.pos < b.len() && b[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
    fn token(&mut self) -> Option<&'a str> {
        self.skip_ws();
        let b = self.s.as_bytes();
        let start = self.pos;
        while self.pos < b.len() && !b[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        (self.pos > start).then(|| &self.s[start..self.pos])
    }
    fn peek(&mut self) -> Option<&'a str> {
        let p = self.pos;
        let t = self.token();
        self.pos = p;
        t
    }
    /// next non-blank character (flags are written separated by blanks, but be tolerant)
    fn ch(&mut self) -> Option<char> {
        self.skip_ws();
        let c = self.s[self.pos..].chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }
    /// rest of the current line (without newline), like fgets
    fn rest_of_line(&mut self) -> &'a str {
        let rest = &self.s[self.pos..];
        let end = rest.find('\n').unwrap_or(rest.len());
        self.pos += (end + 1).min(rest.len());
        rest[..end].trim_end_matches('\r')
    }
    fn line(&self) -> usize {
        self.s.as_bytes()[..self.pos].iter().filter(|&&c| c == b'\n').count() + 1
    }
    fn err<T>(&self, msg: impl std::fmt::Display) -> Result<T, String> {
        Err(format!("line {}: {}", self.line(), msg))
    }
    fn int<T: std::str::FromStr>(&mut self, what: &str) -> Result<T, String> {
        match self.token().map(|t| t.parse::<T>()) {
            Some(Ok(v)) => Ok(v),
            _ => self.err(format!("expected integer for {what}")),
        }
    }
}

pub fn parse(text: &str) -> Result<Config, String> {
    let mut sc = Scan { s: text, pos: 0 };
    let mut root = parse_struct(&mut sc)?;

    // a repeated DATA line is tolerated
    match sc.token() {
        Some("DATA") => {}
        t => return sc.err(format!("expected DATA after the structure, found {:?}", t.unwrap_or("end of file"))),
    }
    while sc.peek() == Some("DATA") {
        sc.token();
    }
    parse_data(&mut sc, &mut root, 1)?;

    let sim_num: u32 = keyword_int(&mut sc, "SIM_NUM")?;
    let seed: u32 = keyword_int(&mut sc, "SEED")?;
    // LSD 8.1 rejects SEED 0 (older example files have it); accepted here, the engine decides
    if sim_num == 0 {
        return sc.err("SIM_NUM must be positive");
    }
    if sc.token() != Some("MAX_STEP") {
        return sc.err("expected MAX_STEP");
    }
    let nums: Vec<i32> = sc.rest_of_line().split_whitespace().take(8).map_while(|t| t.parse().ok()).collect();
    if nums.is_empty() || nums[0] <= 0 {
        return sc.err("MAX_STEP needs a positive integer");
    }
    if sc.token() != Some("EQUATION") {
        return sc.err("expected EQUATION");
    }
    let eq = sc.rest_of_line();
    let equation_file = eq.strip_prefix(' ').unwrap_or(eq).trim_end().to_string();
    // MODELREPORT is read leniently: nothing after it is needed
    let model_report = match sc.token() {
        Some("MODELREPORT") => sc.token().unwrap_or("").to_string(),
        _ => String::new(),
    };

    Ok(Config {
        root,
        sim_num,
        seed,
        max_step: nums[0] as u32,
        max_step_extra: nums[1..].to_vec(),
        equation_file,
        model_report,
    })
}

fn keyword_int<T: std::str::FromStr>(sc: &mut Scan, kw: &str) -> Result<T, String> {
    if sc.token() != Some(kw) {
        return sc.err(format!("expected {kw}"));
    }
    sc.int(kw)
}

/// Structure block: `Label X { Son: Y Label Y {...} Var: a Param: b Func: c }`
fn parse_struct(sc: &mut Scan) -> Result<ObjSpec, String> {
    loop {
        match sc.token() {
            Some("Label") => break,
            Some(_) => {}
            None => return sc.err("no 'Label' found: not an LSD configuration"),
        }
    }
    let label = match sc.token() {
        Some(l) => l.to_string(),
        None => return sc.err("missing object label after 'Label'"),
    };
    let mut o = ObjSpec { label, to_compute: true, counts: vec![], elems: vec![], sons: vec![] };
    loop {
        let t = match sc.token() {
            Some(t) => t,
            None => return sc.err(format!("unterminated object '{}' (missing '}}')", o.label)),
        };
        let kind = match t {
            "}" => return Ok(o),
            "Son:" => {
                if sc.token().is_none() {
                    return sc.err("missing label after 'Son:'");
                }
                o.sons.push(parse_struct(sc)?);
                continue;
            }
            "Var:" => Kind::Var,
            "Param:" => Kind::Param,
            "Func:" => Kind::Func,
            _ => continue, // '{' and anything else is skipped, as in LSD
        };
        let label = match sc.token() {
            Some(l) => l.to_string(),
            None => return sc.err("missing element label"),
        };
        o.elems.push(ElemSpec {
            label,
            kind,
            num_lag: 0,
            save: false,
            savei: false,
            debug: 'n',
            plot: false,
            parallel: false,
            initialized: true,
            upd: None,
            values: vec![],
        });
    }
}

/// DATA block for `o` and (recursively) its sons; `n_parents` = instances of the parent type.
fn parse_data(sc: &mut Scan, o: &mut ObjSpec, n_parents: usize) -> Result<(), String> {
    if sc.token() != Some("Object:") {
        return sc.err(format!("DATA: expected 'Object: {}'", o.label));
    }
    match sc.token() {
        Some(l) if l == o.label => {}
        l => return sc.err(format!("DATA: expected object '{}', found '{}'", o.label, l.unwrap_or(""))),
    }
    o.to_compute = sc.ch() == Some('C');
    for _ in 0..n_parents {
        let n: usize = sc.int(&format!("instance count of '{}'", o.label))?;
        // LSD's replicate() never goes below one instance, so 0 cannot be a valid count
        if n == 0 {
            return sc.err(format!("DATA: object '{}' has instance count 0", o.label));
        }
        o.counts.push(n);
    }
    let total = o.total();

    for e in &mut o.elems {
        let ctx = format!("element '{}' of '{}'", e.label, o.label);
        let want = match e.kind {
            Kind::Var => "Var:",
            Kind::Param => "Param:",
            Kind::Func => "Func:",
        };
        if sc.token() != Some(want) {
            return sc.err(format!("DATA: {ctx} expected '{want}' line"));
        }
        match sc.token() {
            Some(l) if l == e.label => {}
            l => return sc.err(format!("DATA: expected {ctx}, found '{}'", l.unwrap_or(""))),
        }
        e.num_lag = sc.int(&format!("num_lag of {ctx}"))?;
        let (c1, c2, c3, c4) = match (sc.ch(), sc.ch(), sc.ch(), sc.ch()) {
            (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
            _ => return sc.err(format!("DATA: {ctx} is missing its option flags")),
        };
        e.save = c1.to_ascii_lowercase() == 's';
        e.savei = c1 == 'S' || c1 == 'N';
        e.initialized = c2 != '-';
        e.debug = c3;
        e.plot = c4.to_ascii_lowercase() == 'p';
        e.parallel = c4 == 'P' || c4 == 'N';

        let per = if e.kind == Kind::Param { 1 } else { e.num_lag };
        for _ in 0..total {
            let mut v = Vec::with_capacity(per);
            for _ in 0..per {
                match sc.peek().and_then(|t| t.parse::<f64>().ok()) {
                    Some(x) => {
                        sc.token();
                        v.push(x);
                    }
                    None => {
                        let found = e.values.iter().map(Vec::len).sum::<usize>() + v.len();
                        return sc.err(format!("DATA: {ctx} expected {} values, found {found}", total * per));
                    }
                }
            }
            e.values.push(v);
        }

        if e.kind == Kind::Var && sc.peek().is_some_and(|t| t.starts_with("<upd:")) {
            sc.token();
            let mut u = [0i32; 4];
            for x in &mut u {
                let t = sc.token().unwrap_or("");
                *x = match t.trim_end_matches('>').parse() {
                    Ok(v) => v,
                    Err(_) => return sc.err(format!("DATA: bad <upd: ...> for {ctx}")),
                };
            }
            if sc.peek() == Some(">") {
                sc.token();
            }
            e.upd = Some(u);
        }
    }

    for s in &mut o.sons {
        parse_data(sc, s, total)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = "\t\nLabel Root\n{\n\tSon: Firm\n\tLabel Firm\n\t{\n\t\tSon: Worker\n\t\tLabel Worker\n\t\t{\n\t\t\tParam: skill\n\t\t}\n\t\tVar: out\n\t\tParam: size\n\t}\n\tVar: total\n\tFunc: f\n}\n\nDATA\n\nObject: Root C\t1\nVar: total 2 s + n n\t1.5\t2.5\nFunc: f 1 n + n n\t7\n\nObject: Firm C\t3\nVar: out 1 S + n p\t1\t2\t3\t<upd: 1 2 3 4>\nParam: size 0 n + n n\t10\t20\t30\n\nObject: Worker N\t1\t2\t1\nParam: skill 0 n + n n\t0.1\t0.2\t0.3\t0.4\n\nSIM_NUM 2\nSEED 7\nMAX_STEP 100 5\nEQUATION fun x.cpp\nMODELREPORT report_x.html\n\nDESCRIPTION\n";

    #[test]
    fn small_config() {
        let c = parse(SMALL).unwrap();
        assert_eq!((c.sim_num, c.seed, c.max_step), (2, 7, 100));
        assert_eq!(c.max_step_extra, vec![5]);
        assert_eq!(c.equation_file, "fun x.cpp");
        assert_eq!(c.root.counts, vec![1]);
        let t = &c.root.elems[0];
        assert!(t.save && !t.savei && t.initialized);
        assert_eq!(t.values, vec![vec![1.5, 2.5]]);
        assert_eq!(c.root.elems[1].kind, Kind::Func);
        let firm = &c.root.sons[0];
        assert_eq!(firm.counts, vec![3]);
        let out = &firm.elems[0];
        assert!(out.save && out.savei && out.plot && !out.parallel);
        assert_eq!(out.upd, Some([1, 2, 3, 4]));
        assert_eq!(out.values, vec![vec![1.0], vec![2.0], vec![3.0]]);
        assert_eq!(firm.elems[1].values, vec![vec![10.0], vec![20.0], vec![30.0]]);
        let w = &firm.sons[0];
        assert!(!w.to_compute);
        assert_eq!(w.counts, vec![1, 2, 1]);
        assert_eq!(w.total(), 4);
        assert_eq!(w.elems[0].values.len(), 4);
    }

    #[test]
    fn repeated_data_tolerated() {
        parse(&SMALL.replace("DATA\n", "DATA\nDATA\n")).unwrap();
    }

    #[test]
    fn short_param_values() {
        let e = parse(&SMALL.replace("\t0.3\t0.4", "\t0.3")).unwrap_err();
        assert!(e.contains("element 'skill' of 'Worker' expected 4 values, found 3"), "{e}");
    }

    #[test]
    fn missing_data_and_bad_label() {
        assert!(parse("Label Root\n{\n}\n").unwrap_err().contains("DATA"));
        let e = parse(&SMALL.replace("Object: Firm", "Object: Frim")).unwrap_err();
        assert!(e.contains("expected object 'Firm'"), "{e}");
    }
}
