#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("window-rectangle is Windows-only");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    Ok(())
}
