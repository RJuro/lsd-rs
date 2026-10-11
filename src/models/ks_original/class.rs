//! fun_KS_class.h: the C++ classes, constants and accessor macros of the K+S model.
use std::collections::BTreeMap;

use crate::model::{Ctx, ObjId};

/// `struct countryE`: the Country object's extension. The C++ keeps `object *` pointers to the market containers
/// and two speed-up containers of sector 2 firms (`firm2ptr` is only filled, never read; kept for fidelity).
/// `firm2map` is a `std::map<int, object *>` (ordered by firm ID, not by pointer).
pub struct CountryE {
    pub fin_sec: ObjId,
    pub cap_sec: ObjId,
    pub con_sec: ObjId,
    pub lab_sup: ObjId,
    pub mac_sta: ObjId,
    pub sec_sta: ObjId,
    pub lab_sta: ObjId,
    pub firm2ptr: Vec<ObjId>,
    pub firm2map: BTreeMap<i32, ObjId>,
}

// initial notional definitions
pub const INIPROD: f64 = 1.0;
pub const INIWAGE: f64 = 1.0;

// number of dynamic hooks of Firm2, and their names
pub const FIRM2HK: usize = 2;
pub const SUPPL: usize = 0;
pub const TOPVINT: usize = 1;

// Object-location macros (CAPSECL0 ...): the container objects of the country, found through the country extension
// from the calling object (L0), its parent (L1) or its grandparent (L2).
macro_rules! locator {
    ($l0:ident, $l1:ident, $l2:ident, $field:ident) => {
        #[allow(dead_code)] // the C++ defines all three levels for every container; not every one is used
        pub fn $l0(c: &Ctx) -> ObjId { c.ext::<CountryE>(c.p).$field }
        #[allow(dead_code)]
        pub fn $l1(c: &Ctx) -> ObjId { c.ext::<CountryE>(c.parent()).$field }
        #[allow(dead_code)]
        pub fn $l2(c: &Ctx) -> ObjId { c.ext::<CountryE>(c.grandparent()).$field }
    };
}
locator!(capsecl0, capsecl1, capsecl2, cap_sec);
locator!(consecl0, consecl1, consecl2, con_sec);
locator!(labsupl0, labsupl1, labsupl2, lab_sup);
locator!(finsecl0, finsecl1, finsecl2, fin_sec);
locator!(macstal0, macstal1, macstal2, mac_sta);
locator!(secstal0, secstal1, secstal2, sec_sta);
locator!(labstal0, labstal1, labstal2, lab_sta);

/// LABSUPL3: the labor supply of the country, from a Vint object (PARENTS( GRANDPARENT )).
pub fn labsupl3(c: &Ctx) -> ObjId {
    let g = c.grandparent();
    c.ext::<CountryE>(c.up(g)).lab_sup
}

/// V_EXTS( country, countryE, field ) for an arbitrary country object.
pub fn country_e<'a>(c: &'a Ctx, country: ObjId) -> &'a CountryE {
    c.ext::<CountryE>(country)
}

/// Macro VNT: pack vintage time and supplier ID.
pub fn vnt(t0: f64, id_suppl: f64) -> i32 {
    10000 * (t0.trunc() as i32) + (id_suppl.trunc() as i32)
}
