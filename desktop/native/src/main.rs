#[cfg(feature = "host")]
fn main() {
    masterdata_desktop::host::run();
}
#[cfg(not(feature = "host"))]
fn main() {
    eprintln!("Build Desktop with --features host");
}
