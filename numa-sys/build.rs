fn main() {
    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search=/usr/lib");
    println!("cargo:rustc-link-search=/usr/lib/x86_64-linux-gnu");

    println!("cargo:rustc-link-lib=numa");
}
