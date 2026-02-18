//! Print information about the system and agent-chain packages for debugging purposes.
//!
//! This module provides utilities for printing system and package information,
//! useful for debugging and support purposes. It is a Rust adaptation of
//! langchain_core's sys_info.py module.

use crate::env::VERSION;
use rustc_version_runtime::version;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Write};

/// Information about a package/crate.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// The package name.
    pub name: Cow<'static, str>,
    /// The package version, if available.
    pub version: Option<&'static str>,
}

impl PackageInfo {
    /// Create a new PackageInfo with a static name.
    pub fn new_static(name: &'static str, version: Option<&'static str>) -> Self {
        Self {
            name: Cow::Borrowed(name),
            version,
        }
    }

    /// Create a new PackageInfo with an owned name.
    pub fn new_owned(name: String, version: Option<&'static str>) -> Self {
        Self {
            name: Cow::Owned(name),
            version,
        }
    }
}

/// System information for debugging.
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// Operating system name (e.g., "linux", "macos", "windows").
    pub os: &'static str,
    /// Operating system family (e.g., "unix", "windows").
    pub os_family: &'static str,
    /// CPU architecture (e.g., "x86_64", "aarch64").
    pub arch: &'static str,
    /// Rust version used to compile.
    pub rust_version: String,
}

impl SystemInfo {
    /// Get the current system information.
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS,
            os_family: std::env::consts::FAMILY,
            arch: std::env::consts::ARCH,
            rust_version: version().to_string(),
        }
    }
}

/// Get sub-dependencies that are not in the main package list.
///
/// In Rust, dependencies are determined at compile time, so this function
/// returns a predefined list of common dependencies used by the agent-chain crates.
fn get_sub_deps(packages: &[Cow<'static, str>]) -> Vec<PackageInfo> {
    let all_deps = [
        ("async-trait", option_env!("DEP_ASYNC_TRAIT_VERSION")),
        ("futures", option_env!("DEP_FUTURES_VERSION")),
        ("reqwest", option_env!("DEP_REQWEST_VERSION")),
        ("serde", option_env!("DEP_SERDE_VERSION")),
        ("serde_json", option_env!("DEP_SERDE_JSON_VERSION")),
        ("tokio", option_env!("DEP_TOKIO_VERSION")),
        ("tracing", option_env!("DEP_TRACING_VERSION")),
    ];

    let package_set: std::collections::HashSet<&str> =
        packages.iter().map(|s| s.as_ref()).collect();

    all_deps
        .iter()
        .filter(|(name, _)| !package_set.contains(name))
        .map(|(name, version)| PackageInfo::new_static(name, *version))
        .collect()
}

/// Get information about agent-chain packages.
///
/// Returns a list of package information for all agent-chain related crates.
pub fn get_package_info() -> Vec<PackageInfo> {
    let mut packages = vec![
        PackageInfo::new_static("agent-chain-core", Some(VERSION)),
        PackageInfo::new_static("agent-chain", option_env!("DEP_AGENT_CHAIN_VERSION")),
        PackageInfo::new_static("agent-graph", option_env!("DEP_AGENT_GRAPH_VERSION")),
        PackageInfo::new_static(
            "agent-chain-macros",
            option_env!("DEP_AGENT_CHAIN_MACROS_VERSION"),
        ),
        PackageInfo::new_static(
            "agent-graph-macros",
            option_env!("DEP_AGENT_GRAPH_MACROS_VERSION"),
        ),
    ];

    if packages.len() > 1 {
        packages[1..].sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    packages
}

/// Get all system and package information as structured data.
///
/// Returns a tuple of (system_info, packages, sub_dependencies).
pub fn get_sys_info(additional_pkgs: &[&str]) -> (SystemInfo, Vec<PackageInfo>, Vec<PackageInfo>) {
    let system_info = SystemInfo::current();
    let mut packages = get_package_info();

    for pkg in additional_pkgs {
        packages.push(PackageInfo::new_owned((*pkg).to_string(), None));
    }

    let package_names: Vec<Cow<'static, str>> = packages.iter().map(|p| p.name.clone()).collect();
    let sub_deps = get_sub_deps(&package_names);

    (system_info, packages, sub_deps)
}

/// Print information about the environment for debugging purposes.
///
/// This function prints system information, package versions, and dependencies
/// to stdout. It is useful for debugging and support purposes.
///
/// # Arguments
///
/// * `additional_pkgs` - Additional package names to include in the output.
///
/// # Example
///
/// ```
/// use agent_chain_core::sys_info::print_sys_info;
///
/// print_sys_info(&[]);
/// ```
pub fn print_sys_info(additional_pkgs: &[&str]) {
    let mut stdout = io::stdout();
    print_sys_info_to(&mut stdout, additional_pkgs);
}

/// Print system information to a specific writer.
///
/// This is useful for testing or redirecting output.
pub fn print_sys_info_to<W: Write>(writer: &mut W, additional_pkgs: &[&str]) {
    let (system_info, packages, sub_deps) = get_sys_info(additional_pkgs);

    writeln!(writer).ok();
    writeln!(writer, "System Information").ok();
    writeln!(writer, "------------------").ok();
    writeln!(writer, "> OS:  {}", system_info.os).ok();
    writeln!(writer, "> OS Family:  {}", system_info.os_family).ok();
    writeln!(writer, "> Architecture:  {}", system_info.arch).ok();
    writeln!(writer, "> Rust Version:  {}", system_info.rust_version).ok();

    writeln!(writer).ok();
    writeln!(writer, "Package Information").ok();
    writeln!(writer, "-------------------").ok();

    let mut not_installed: Vec<&str> = Vec::new();

    for pkg in &packages {
        match pkg.version {
            Some(version) => {
                writeln!(writer, "> {}: {}", pkg.name, version).ok();
            }
            None => {
                not_installed.push(pkg.name.as_ref());
            }
        }
    }

    if !not_installed.is_empty() {
        writeln!(writer).ok();
        writeln!(writer, "Optional packages not installed").ok();
        writeln!(writer, "-------------------------------").ok();
        for pkg in not_installed {
            writeln!(writer, "> {}", pkg).ok();
        }
    }

    let deps_with_version: Vec<_> = sub_deps.iter().filter(|d| d.version.is_some()).collect();

    if !deps_with_version.is_empty() {
        writeln!(writer).ok();
        writeln!(writer, "Other Dependencies").ok();
        writeln!(writer, "------------------").ok();

        for dep in deps_with_version {
            if let Some(version) = dep.version {
                writeln!(writer, "> {}: {}", dep.name, version).ok();
            }
        }
    }
}

/// Get system information as a HashMap (for compatibility with env module).
pub fn get_sys_info_map() -> HashMap<String, String> {
    let system_info = SystemInfo::current();
    let packages = get_package_info();

    let mut map = HashMap::new();
    map.insert("os".to_string(), system_info.os.to_string());
    map.insert("os_family".to_string(), system_info.os_family.to_string());
    map.insert("arch".to_string(), system_info.arch.to_string());
    map.insert(
        "rust_version".to_string(),
        system_info.rust_version.to_string(),
    );

    for pkg in packages {
        if let Some(version) = pkg.version {
            map.insert(
                format!("{}_version", pkg.name.replace('-', "_")),
                version.to_string(),
            );
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_current() {
        let info = SystemInfo::current();

        assert!(!info.os.is_empty());
        assert!(!info.os_family.is_empty());
        assert!(!info.arch.is_empty());
        assert!(!info.rust_version.is_empty());
    }

    #[test]
    fn test_get_package_info() {
        let packages = get_package_info();

        assert!(!packages.is_empty());

        assert_eq!(packages[0].name, "agent-chain-core");
        assert!(packages[0].version.is_some());
    }

    #[test]
    fn test_get_sys_info() {
        let (system_info, packages, _sub_deps) = get_sys_info(&[]);

        assert!(!system_info.os.is_empty());
        assert!(!packages.is_empty());
    }

    #[test]
    fn test_get_sys_info_with_additional_pkgs() {
        let (_, packages, _) = get_sys_info(&["custom-package"]);

        let has_custom = packages.iter().any(|p| p.name == "custom-package");
        assert!(has_custom);
    }

    #[test]
    fn test_print_sys_info_to_buffer() {
        let mut buffer = Vec::new();
        print_sys_info_to(&mut buffer, &[]);

        let output = String::from_utf8(buffer).unwrap();

        assert!(output.contains("System Information"));
        assert!(output.contains("Package Information"));
        assert!(output.contains("agent-chain-core"));
    }

    #[test]
    fn test_get_sys_info_map() {
        let map = get_sys_info_map();

        assert!(map.contains_key("os"));
        assert!(map.contains_key("rust_version"));
        assert!(map.contains_key("agent_chain_core_version"));
    }
}
