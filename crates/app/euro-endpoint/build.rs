//! Bake `BACKEND_URL` into `DEFAULT_API_URL` at compile time. See
//! `euro-settings/build.rs` for the same pattern with more
//! commentary; the two scripts deliberately stay independent (no
//! shared build crate) — the forwarding logic is short enough to
//! inline.

const REQUIRED: &[&str] = &["BACKEND_URL"];

fn main() {
    for key in REQUIRED {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| missing(key));
        println!("cargo:rustc-env={key}={value}");
    }
}

fn missing(key: &str) -> ! {
    panic!(
        "build.rs: required env var `{key}` is unset.\n\
         Build via `just <recipe>` — the justfile loads `.env` and exports\n\
         every variable to cargo. To run `cargo build` directly, export\n\
         `{key}` first (`set -a; source .env; set +a; cargo build …`) or\n\
         use `direnv` (the repo ships an `.envrc`)."
    );
}
