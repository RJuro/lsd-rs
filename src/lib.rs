//! A Rust reimplementation of the LSD (Laboratory for Simulation Development) engine.
//! Compat mode: reproduces LSD 8.1's computations bit for bit, checked against the original.
pub mod cli;
pub mod config;
pub mod macros;
pub mod mcp;
pub mod model;
pub mod nets;
pub mod models;
pub mod output;
pub mod rng;
