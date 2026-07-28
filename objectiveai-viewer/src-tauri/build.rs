fn main() {
    // DELAY-LOAD `libcef.dll`.
    //
    // This single link flag is what lets the Chromium runtime be
    // fetched on demand instead of shipped: without it the loader
    // resolves libcef.dll at process start, so a viewer that has never
    // opened a browser tab — and therefore has no runtime on disk —
    // would fail to launch at all. Delay-loaded, the DLL is not touched
    // until the first CEF call, by which point `cef::ensure_runtime`
    // has downloaded it and `AddDllDirectory` has made it findable.
    //
    // `delayimp.lib` carries the resolver stub those calls route
    // through; MSVC does not link it implicitly.
    #[cfg(windows)]
    {
        println!("cargo::rustc-link-arg-bins=/DELAYLOAD:libcef.dll");
        println!("cargo::rustc-link-arg-bins=delayimp.lib");
    }
    tauri_build::build()
}
