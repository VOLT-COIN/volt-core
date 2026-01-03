use std::env;
use std::path::Path;

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let res_path = Path::new(&dir).join("resources.o");
    println!("cargo:rustc-link-arg={}", res_path.display());
}
