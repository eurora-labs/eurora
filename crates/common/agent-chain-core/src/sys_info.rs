use crate::env::VERSION;
use rustc_version_runtime::version;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: Cow<'static, str>,
    pub version: Option<&'static str>,
}

impl PackageInfo {
    pub fn new_static(name: &'static str, version: Option<&'static str>) -> Self {
        Self {
            name: Cow::Borrowed(name),
            version,
        }
    }

    pub fn new_owned(name: String, version: Option<&'static str>) -> Self {
        Self {
            name: Cow::Owned(name),
            version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: &'static str,
    pub os_family: &'static str,
    pub arch: &'static str,
    pub rust_version: String,
}

impl SystemInfo {
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS,
            os_family: std::env::consts::FAMILY,
            arch: std::env::consts::ARCH,
            rust_version: version().to_string(),
        }
    }
}

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

pub fn print_sys_info(additional_pkgs: &[&str]) {
    let mut stdout = io::stdout();
    print_sys_info_to(&mut stdout, additional_pkgs);
}

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
