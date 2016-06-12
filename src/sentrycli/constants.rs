pub const DEFAULT_URL : &'static str = "https://app.getsentry.com/";
pub const PROTOCOL_VERSION : u32 = 6;
pub const VERSION : &'static str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/constants.gen.rs"));
