//! HSD readers derived from the format declarations in Melee's baselib headers.
//! No disc data is compiled into this crate.
pub mod action;
pub mod animation;
pub mod audio;
pub mod dat;
pub mod disc;
pub mod model;
pub mod params;
pub mod presentation;
pub mod texture;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub const COSTUMES: [&str; 5] = ["Nr", "Pi", "Bk", "Gr", "La"];
pub const REQUIRED: [&str; 13] = [
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
    "audio/us/samus.ssm",
    "audio/us/main.ssm",
    "audio/us/smash2.sem",
];

pub fn validate_asset(name: &str, bytes: &[u8]) -> Result<()> {
    if name.ends_with(".ssm") {
        audio::ssm(bytes)?;
    } else if name.ends_with(".sem") {
        audio::Bank::sequence(bytes)?;
    } else if name == "PlSsAJ.dat" {
        if bytes.len() <= 32 {
            return Err("truncated animation archive".into());
        }
    } else {
        dat::Dat::parse(bytes)?;
    }
    Ok(())
}

pub fn complete(path: &std::path::Path) -> bool {
    REQUIRED.iter().all(|name| {
        std::fs::read(path.join(name))
            .ok()
            .is_some_and(|bytes| validate_asset(name, &bytes).is_ok())
    })
}
