// build.rs
#[cfg(windows)]
fn main() {
    // pack assets/icon.ico as the exe icon (Windows resource)
    if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile()?;
        Ok(())
    })() {
        println!("cargo:warning=winres failed: {}", e);
    }
}

#[cfg(not(windows))]
fn main() {}
