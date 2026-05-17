//! Eurora backend entry point.
//!
//! Hands off to [`bootstrap::run`] for everything fallible; this file owns
//! only the tokio runtime lifecycle and the pretty-printer for startup
//! errors. Keeping `main` thin means a single `?` in `bootstrap` can carry
//! any [`errors::BootstrapError`] variant out to the user.

mod bootstrap;
mod errors;

use std::process::ExitCode;

use crate::errors::BootstrapError;

fn main() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to start tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(bootstrap::run()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            print_startup_error(&e);
            ExitCode::FAILURE
        }
    }
}

/// Print the bootstrap error outside the tracing pipeline.
///
/// We deliberately use `eprintln!` rather than `tracing::error!` here:
/// startup failures can include "tracing init failed" or happen before the
/// subscriber is fully wired, and we want the message to land on stderr
/// either way. The blank-line padding makes the message visible in a
/// terminal full of cargo / docker compose chatter.
fn print_startup_error(err: &BootstrapError) {
    eprintln!();
    eprintln!("─── Eurora backend failed to start ───");
    eprintln!();
    eprintln!("{err}");
    eprintln!();
    eprintln!(
        "See `crates/backend/be-monolith/README.md` for setup, or run `just doctor` \
         to check your environment."
    );
    eprintln!();
}
