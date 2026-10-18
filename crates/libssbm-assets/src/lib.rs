//! HSD readers derived from the format declarations in Melee's baselib headers.
//! No disc data is compiled into this crate.
pub mod action;
pub mod animation;
pub mod article;
pub mod audio;
pub mod dat;
pub mod disc;
pub mod effects;
pub mod fighter;
pub mod grapple;
pub mod model;
pub mod params;
pub mod portrait;
pub mod presentation;
pub mod texture;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub use fighter::COSTUMES;
pub const REQUIRED: [&str; 15] = [
    "PlSs.dat",
    "PlCo.dat",
    "PlSsAJ.dat",
    "PlSsNr.dat",
    "PlSsPi.dat",
    "PlSsBk.dat",
    "PlSsGr.dat",
    "PlSsLa.dat",
    "EfSsData.dat",
    "EfCoData.dat",
    "MnSlChr.usd",
    "audio/us/samus.ssm",
    "audio/us/main.ssm",
    "audio/us/nr_name.ssm",
    "audio/us/smash2.sem",
];

pub fn validate_asset(name: &str, bytes: &[u8]) -> Result<()> {
    fighter::SAMUS.validate_asset(name, bytes)
}

pub fn complete(path: &std::path::Path) -> bool {
    fighter::SAMUS.complete(path)
}
