//! Bake `BACKEND_URL` into `api_settings::DEFAULT_API_URL` at compile time.
//!
//! The value comes from the process environment. The justfile (`set
//! dotenv-load`) is the single point that reads `.env` and exports it
//! into cargo; CI and production deployments inject vars via their
//! own mechanisms. There is no in-source fallback — fork-and-rebrand
//! builds override this by editing `.env` or exporting in CI.

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
