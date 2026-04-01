fn main() {
    // Set NU_FEATURES env var at compile time (used by version command)
    let features: Vec<&str> = vec![
        #[cfg(feature = "plugin")]
        "plugin",
        #[cfg(feature = "sqlite")]
        "sqlite",
        #[cfg(feature = "trash-support")]
        "trash-support",
        #[cfg(feature = "network")]
        "network",
        #[cfg(feature = "mcp")]
        "mcp",
    ];
    println!("cargo:rustc-env=NU_FEATURES={}", features.join(","));

    // Extract nushell version from nu-protocol's Cargo.toml
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let nu_cargo_path = std::path::Path::new(&manifest_dir)
        .join("nushell/crates/nu-protocol/Cargo.toml");
    let nu_version = std::fs::read_to_string(&nu_cargo_path)
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                line.strip_prefix("version = \"")?.strip_suffix('"').map(String::from)
            })
        })
        .unwrap_or_else(|| "0.111.0".to_string());
    println!("cargo:rustc-env=NUSHELL_VERSION={nu_version}");
    println!("cargo:rerun-if-changed=nushell/crates/nu-protocol/Cargo.toml");
}
