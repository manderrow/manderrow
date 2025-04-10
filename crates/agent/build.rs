pub fn main() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        println!(
            "cargo::rustc-link-search={}/dll_proxy",
            std::env::var("CARGO_MANIFEST_DIR").unwrap()
        );
    }
}
