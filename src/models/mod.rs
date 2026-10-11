//! Model equation files, ported from LSD's C++ (Example/Test). Each returns its equations by label.
use crate::model::EqDef;

pub mod bugs_life;
pub mod game_of_life;
pub mod ks_original;
pub mod logistic_chaos;
pub mod macro_tests;
pub mod nets;
pub mod random_walk;
pub mod repl_dyn_macro;
pub mod worm;

pub fn get(name: &str) -> Option<Vec<(&'static str, EqDef)>> {
    Some(match name {
        "BugsLife" => bugs_life::equations(),
        "GameOfLife" => game_of_life::equations(),
        "K+S-original" => ks_original::equations(),
        "LogisticChaos" => logistic_chaos::equations(),
        "Nets" => nets::equations(),
        "RandomWalk" => random_walk::equations(),
        "ReplDynMacro" => repl_dyn_macro::equations(),
        "Worm" => worm::equations(),
        _ => return nets::get(name).or_else(|| macro_tests::get(name)),
    })
}

pub const NAMES: &[&str] = &["BugsLife", "GameOfLife", "K+S-original", "LogisticChaos", "Nets", "NetsFixed", "NetsGen", "NetsOps", "RandomWalk", "ReplDynMacro", "Worm",
    "MacroStats", "MacroSort", "MacroSearch", "MacroSearchFast", "MacroObjects", "MacroHooks"];
