#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    let _ = res.compile();

    println!("cargo:rustc-link-args=/SUBSYSTEM:WINDOWS");
}

#[cfg(not(windows))]
fn main() {}
