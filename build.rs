use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;


fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("constants.gen.rs");
    let mut f = File::create(&dest_path).unwrap();

    let target = env::var("TARGET").unwrap();
    let mut target_bits = target.split('-');
    let arch = target_bits.next().unwrap();
    target_bits.next();
    let platform = target_bits.next().unwrap();

    write!(f, "pub const PLATFORM : &'static str = \"{}\";\n", platform).ok();
    write!(f, "pub const ARCH : &'static str = \"{}\";\n", arch).ok();

    // we need this when linking openssl statically
    if platform == "windows" {
        println!("cargo:rustc-link-lib=gdi32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=advapi32");
    }
}
