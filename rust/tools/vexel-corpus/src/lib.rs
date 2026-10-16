#![forbid(unsafe_code)]
//! Developer tool: runs the Rust decode+render pipeline over a pinned corpus
//! manifest, one isolated child process per file, and writes verdict reports.
//! Measurement only: no fidelity, conformance or performance claim.

pub mod cli;
pub mod compare;
pub mod jsonx;
pub mod manifest;
pub mod report;
pub mod run;
pub mod runner;
pub mod verdict;
