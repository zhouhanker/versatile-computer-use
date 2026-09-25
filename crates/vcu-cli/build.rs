fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }
    // MSVC link.exe uses /STACK. GNU ld treats that token as a filename.
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if env == "gnu" {
        println!("cargo:rustc-link-arg=-Wl,--stack,8388608");
    } else {
        println!("cargo:rustc-link-arg=/STACK:8388608");
    }
}
