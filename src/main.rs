//! lsd-rs: run LSD models headless, for people and agents. Every command prints JSON; see `lsd-rs help`.
fn main() {
    lsd::cli::main_with(lsd::models::get, lsd::models::NAMES);
}
