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
}
