use std::path::PathBuf;

fn main() {
    let release_dir = PathBuf::from("cef/windows/Release");
    
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/feather.ico"); // path to your .ico
        res.compile().unwrap();
    }
    
    println!(
        "cargo:rustc-link-search=native={}",
        release_dir.display()
    );

    println!("cargo:rustc-link-lib=dylib=libcef");
    println!("cargo:rerun-if-changed=cef/");
}