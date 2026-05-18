//! Bake `BACKEND_URL` into `DEFAULT_API_URL` at compile time. See
//! `euro-settings/build.rs` for the same pattern with more
//! commentary; the two scripts deliberately stay independent (no
//! shared build crate) — the forwarding logic is short enough to
//! inline.

const VARS: &[(&str, &str)] = &[("BACKEND_URL", "http://localhost:3000")];

fn main() {
    for (key, default) in VARS {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| (*default).to_owned());
        println!("cargo:rustc-env={key}={value}");
    }
}
