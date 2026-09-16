//! Ensures the embedded dashboard directory exists so rust-embed compiles
//! on a fresh clone where the dashboard has not been built yet.
fn main() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dashboards/agent/dist");
    std::fs::create_dir_all(&dist).ok();
    println!("cargo:rerun-if-changed={}", dist.display());
}
