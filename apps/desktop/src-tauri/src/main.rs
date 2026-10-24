//! ATENA for one computer: the window around `atena-app`.
//!
//! The same program is also the limited worker that parses one untrusted document
//! (`atena_extract::Parser`): started with `atena_extract::WORKER_ARG`, it counts its allocations
//! and stops beyond its memory limit.

// no console window next to the app on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod desktop;
mod logging;

use std::process::ExitCode;

/// Every allocation of this program is counted, so that a parsing worker can be held to its
/// memory limit.
#[global_allocator]
static ALLOCATOR: cap::Cap<std::alloc::System> = cap::Cap::new(std::alloc::System, usize::MAX);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some(atena_extract::WORKER_ARG) {
        return atena_extract::worker_main(args.get(1).map(String::as_str), |limit| {
            ALLOCATOR.set_limit(limit).is_ok()
        });
    }
    match desktop::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ATENA: {e}");
            ExitCode::FAILURE
        }
    }
}
