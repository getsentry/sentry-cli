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

    write!(f, "/// The platform identifier\n").ok();
    write!(f, "pub const PLATFORM: &'static str = \"{}\";\n", platform).ok();
    write!(f, "/// The CPU architecture identifier\n").ok();
    write!(f, "pub const ARCH: &'static str = \"{}\";\n", arch).ok();
    write!(f, "/// The user agent for sentry events\n").ok();
    write!(
        f,
        "pub const USER_AGENT: &'static str = \"sentry-cli/{}\";\n",
        arch
    ).ok();
    println!("cargo:rerun-if-changed=build.rs\n");
}
