use std::path::PathBuf;

fn main() {
    let release_dir = PathBuf::from("cef/windows/Release");

    println!(
        "cargo:rustc-link-search=native={}",
        release_dir.display()
    );

    println!("cargo:rustc-link-lib=dylib=libcef");
    println!("cargo:rerun-if-changed=cef/");
}