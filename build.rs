extern crate bindgen;
use std::env;

fn main() {
    let include_dir = env::var("OPENSSL_INCLUDE_DIR").unwrap_or("/usr/local/include/gc".into());
    let include_path = format!("{}/gc_typed.h", include_dir);

    let bindings = bindgen::Builder::new(&*include_path);
    let generated_bindings = bindings.generate().expect("Failed to generate bindings");
    generated_bindings.write_to_file("src/ffi.rs").expect("failed to write bindings");
}
