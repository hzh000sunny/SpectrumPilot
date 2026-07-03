fn main() {
    println!("cargo:rerun-if-changed=resources/3gpp/catalog_seed");
    tauri_build::build()
}
