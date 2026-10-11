//! Bit-exact replica of LSD 8.1's random numbers (util.cpp) as built with GCC 15.2 / libstdc++:
//! mt19937 main generator (ran_gen_id = 2) and the internal minstd_rand `lc1`.
//! Every LSD call builds a fresh std distribution object, so distribution state
//! (normal's saved value) lives only within one call.

extern "C" {
    fn lgamma(x: f64) -> f64; // same libm as std::lgamma
}

struct Mt19937 { mt: [u32; 624], i: usize }

impl Mt19937 {
    fn new(seed: u32) -> Self {
        let mut mt = [0u32; 624];
        mt[0] = seed;
        for i in 1..624 { mt[i] = 1812433253u32.wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_add(i as u32); }
        Mt19937 { mt, i: 624 }
    }

    fn next_u32(&mut self) -> u32 {
        if self.i >= 624 {
            for k in 0..624 {
                let y = (self.mt[k] & 0x8000_0000) | (self.mt[(k + 1) % 624] & 0x7fff_ffff);
                self.mt[k] = self.mt[(k + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.i = 0;
        }
        let mut y = self.mt[self.i];
        self.i += 1;
        y ^= y >> 11; y ^= (y << 7) & 0x9d2c_5680; y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    // libstdc++ generate_canonical<double, 53> on a 32-bit engine: two draws, low word first
    fn canonical(&mut self) -> f64 {
        let r = 4294967296.0f64;
        let s = self.next_u32() as f64 + self.next_u32() as f64 * r;
        let x = s / (r * r);
        if x >= 1.0 { 1.0f64.next_down() } else { x }
    }
}

/// minstd_rand: x = 48271 x mod (2^31 - 1)
struct MinStd(u32);

impl MinStd {
    fn new(seed: u32) -> Self {
        let s = seed % 2147483647;
        MinStd(if s == 0 { 1 } else { s })
    }
}

/// Uniform random bit generator as seen by uniform_int_distribution (result type uint_fast32_t = u32 here).
trait Urbg {
    const MIN: u32;
    const MAX: u32;
    fn next(&mut self) -> u32;
}

impl Urbg for Mt19937 {
    const MIN: u32 = 0;
    const MAX: u32 = u32::MAX;
    fn next(&mut self) -> u32 { self.next_u32() }
}

impl Urbg for MinStd {
    const MIN: u32 = 1;
    const MAX: u32 = 2147483646;
    fn next(&mut self) -> u32 {
        self.0 = (self.0 as u64 * 48271 % 2147483647) as u32;
        self.0
    }
}

/// libstdc++ (GCC 15) uniform_int_distribution<int>::operator()
fn uniform_int_dist<G: Urbg>(g: &mut G, a: i32, b: i32) -> i32 {
    let range = G::MAX - G::MIN;
    let urange = (b as u32).wrapping_sub(a as u32);
    let ret = if range > urange {
        let uerange = urange + 1;
        if range == u32::MAX {
            // Lemire's method (_S_nd with 64-bit product)
            let mut product = g.next() as u64 * uerange as u64;
            let mut low = product as u32;
            if low < uerange {
                let threshold = uerange.wrapping_neg() % uerange;
                while low < threshold {
                    product = g.next() as u64 * uerange as u64;
                    low = product as u32;
                }
            }
            (product >> 32) as u32
        } else {
            // fallback: rejection + division
            let scaling = range / uerange;
            let past = uerange * scaling;
            let mut r;
            loop {
                r = g.next() - G::MIN;
                if r < past { break; }
            }
            r / scaling
        }
    } else if range < urange {
        // upscaling: high word from a recursive call, low word from one more draw
        loop {
            let uer = range + 1;
            let tmp = uer.wrapping_mul(uniform_int_dist(g, 0, (urange / uer) as i32) as u32);
            let r = tmp.wrapping_add(g.next() - G::MIN);
            if !(r > urange || r < tmp) { break r; }
        }
    } else {
        g.next() - G::MIN
    };
    ret.wrapping_add(a as u32) as i32
}

/// std::normal_distribution (polar method). `saved` is the second value of a pair.
#[derive(Default)]
struct Normal { saved: Option<f64> }

impl Normal {
    /// standard normal before scaling (what operator() computes prior to `* sd + mean`)
    fn raw(&mut self, mt: &mut Mt19937) -> f64 {
        if let Some(s) = self.saved.take() { return s; }
        loop {
            let x = 2.0 * mt.canonical() - 1.0;
            let y = 2.0 * mt.canonical() - 1.0;
            let r2 = x * x + y * y;
            if r2 > 1.0 || r2 == 0.0 { continue; }
            let mult = (-2.0 * r2.ln() / r2).sqrt();
            self.saved = Some(x * mult);
            return y * mult;
        }
    }

    /// default-constructed normal_distribution used inside gamma, lognormal, poisson
    fn std(&mut self, mt: &mut Mt19937) -> f64 { self.raw(mt) * 1.0 + 0.0 }
}

/// libstdc++ gamma_distribution (Marsaglia-Tsang), unit state fresh per call
fn gamma_dist(mt: &mut Mt19937, alpha: f64, beta: f64) -> f64 {
    let malpha = if alpha < 1.0 { alpha + 1.0 } else { alpha };
    let a1 = malpha - 1.0 / 3.0;
    let a2 = 1.0 / (9.0 * a1).sqrt();
    let mut nd = Normal::default();
    let (mut v, mut n);
    loop {
        loop {
            n = nd.std(mt);
            v = 1.0 + a2 * n;
            if v > 0.0 { break; }
        }
        v = v * v * v;
        let u = mt.canonical();
        if !(u > 1.0 - 0.0331 * n * n * n * n && (u.ln() > (0.5 * n * n + a1 * (1.0 - v + v.ln())))) { break; }
    }
    if alpha == malpha { return a1 * v * beta; }
    let mut u;
    loop {
        u = mt.canonical();
        if u != 0.0 { break; }
    }
    u.powf(1.0 / alpha) * a1 * v * beta
}

/// libstdc++ poisson_distribution<int>
fn poisson_dist(mt: &mut Mt19937, mean: f64) -> i32 {
    if mean < 12.0 {
        let thr = (-mean).exp();
        let (mut x, mut prod) = (0i32, 1.0f64);
        loop {
            prod *= mt.canonical();
            x += 1;
            if prod <= thr { break; }
        }
        return x - 1;
    }
    // rejection algorithm (Devroye); parameters as in param_type::_M_initialize
    let pi_4 = 0.7853981633974483096;
    let m = mean.floor();
    let lm_thr = mean.ln();
    let lfm = unsafe { lgamma(m + 1.0) };
    let sm = m.sqrt();
    let dx = (2.0 * m * (32.0 * m / pi_4).ln()).sqrt();
    let d = 6.0f64.max(m.min(dx)).round();
    let cx = 2.0 * m + d;
    let scx = (cx / 2.0).sqrt();
    let inv_cx = 1.0 / cx;
    let c2b = (pi_4 * cx).sqrt() * inv_cx.exp();
    let cb = 2.0 * cx * (-d * inv_cx * (1.0 + d / 2.0)).exp() / d;

    let naf = (1.0 - f64::EPSILON) / 2.0;
    let thr = i32::MAX as f64 + naf;
    let c1 = sm * 1.2533141373155002512078826424055226;
    let c2 = c2b + c1;
    let c3 = c2 + 1.0;
    let c4 = c3 + 1.0;
    let c178 = 0.0128205128205128205128205128205128;
    let c5 = c4 + 1.0129030479320018583185514777512983;
    let c = cb + c5;
    let cx2 = 2.0 * (2.0 * m + d);
    let mut nd = Normal::default();
    loop {
        let u = c * mt.canonical();
        let e = -(1.0 - mt.canonical()).ln();
        let mut w = 0.0f64;
        let x = if u <= c1 {
            let n = nd.std(mt);
            let y = -n.abs() * sm - 1.0;
            let x = y.floor();
            w = -n * n / 2.0;
            if x < -m { continue; }
            x
        } else if u <= c2 {
            let n = nd.std(mt);
            let y = 1.0 + n.abs() * scx;
            let x = y.ceil();
            w = y * (2.0 - y) * inv_cx;
            if x > d { continue; }
            x
        } else if u <= c3 {
            -1.0
        } else if u <= c4 {
            0.0
        } else if u <= c5 {
            w = c178;
            1.0
        } else {
            let v = -(1.0 - mt.canonical()).ln();
            let y = d + v * cx2 / d;
            w = -d * inv_cx * (1.0 + y / 2.0);
            y.ceil()
        };
        let reject = w - e - x * lm_thr > lfm - unsafe { lgamma(x + m + 1.0) } || x + m >= thr;
        if !reject { return (x + m + naf) as i32; }
    }
}

pub struct LsdRng { mt: Mt19937, lc1: MinStd }

impl LsdRng {
    pub fn new(seed: u32) -> Self { LsdRng { mt: Mt19937::new(seed), lc1: MinStd::new(seed) } }

    /// LSD RND: uniform in (0,1)
    pub fn ran1(&mut self) -> f64 {
        loop {
            let r = self.mt.canonical() * (1.0 - 0.0) + 0.0;
            if r != 0.0 { return r; }
        }
    }

    pub fn uniform(&mut self, min: f64, max: f64) -> f64 { self.mt.canonical() * (max - min) + min }

    pub fn uniform_int(&mut self, min: f64, max: f64) -> f64 {
        // LSD casts to long, then uniform_int_distribution<int> narrows to int
        uniform_int_dist(&mut self.mt, min as i64 as i32, max as i64 as i32) as f64
    }

    pub fn rnd_int(&mut self, min: i32, max: i32) -> i32 { uniform_int_dist(&mut self.lc1, min, max) }

    pub fn norm(&mut self, mean: f64, sd: f64) -> f64 {
        if sd < 0.0 { return mean; }
        Normal::default().raw(&mut self.mt) * sd + mean
    }

    pub fn lnorm(&mut self, mean: f64, sd: f64) -> f64 {
        if sd < 0.0 { return mean.exp(); }
        (sd * Normal::default().std(&mut self.mt) + mean).exp()
    }

    pub fn gamma(&mut self, alpha: f64, beta: f64) -> f64 {
        if alpha <= 0.0 || beta <= 0.0 { return 0.0; }
        gamma_dist(&mut self.mt, alpha, beta)
    }

    pub fn beta(&mut self, alpha: f64, beta: f64) -> f64 {
        if alpha <= 0.0 || beta <= 0.0 { return if alpha < beta { 0.0 } else { 1.0 }; }
        let draw = gamma_dist(&mut self.mt, alpha, 1.0);
        draw / (draw + gamma_dist(&mut self.mt, beta, 1.0))
    }

    pub fn poisson(&mut self, mean: f64) -> f64 {
        if mean < 0.0 { return 0.0; }
        poisson_dist(&mut self.mt, mean) as f64
    }

    pub fn bernoulli(&mut self, p: f64) -> f64 {
        if p < 0.0 || p > 1.0 { return if p < 0.0 { 0.0 } else { 1.0 }; }
        if self.mt.canonical() < p * (1.0 - 0.0) { 1.0 } else { 0.0 }
    }
}
