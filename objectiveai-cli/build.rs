fn main() {
    set_stack_size();
}

/// Set the main thread stack size to 16 MB for all supported platforms.
fn set_stack_size() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    let flag = match (os.as_str(), env.as_str()) {
        ("windows", "msvc") => "/STACK:16777216",
        ("windows", "gnu") => "-Wl,--stack,16777216",
        ("macos", _) | ("ios", _) => "-Wl,-stack_size,0x1000000",
        ("linux", _) | ("freebsd", _) | ("netbsd", _) | ("openbsd", _) | ("dragonfly", _) => {
            "-Wl,-z,stacksize=16777216"
        }
        _ => return,
    };

    println!("cargo:rustc-link-arg={flag}");
}
