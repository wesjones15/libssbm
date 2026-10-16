//! HSD readers derived from the format declarations in Melee's baselib headers.
//! No disc data is compiled into this crate.
pub mod dat;
pub mod disc;
pub mod model;
pub mod params;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub const COSTUMES: [&str; 5] = ["Nr", "Pi", "Bk", "Gr", "La"];
pub const REQUIRED: [&str; 8] = [
    "PlSs.dat",
    "PlCo.dat",
    "PlSsAJ.dat",
    "PlSsNr.dat",
    "PlSsPi.dat",
    "PlSsBk.dat",
    "PlSsGr.dat",
    "PlSsLa.dat",
];

pub fn complete(path: &std::path::Path) -> bool {
    REQUIRED.iter().all(|name| {
        std::fs::read(path.join(name)).ok().is_some_and(|bytes| {
            // AJ is a concatenation of animation archives, not one DAT archive.
            if *name == "PlSsAJ.dat" {
                bytes.len() > 32
            } else {
                dat::Dat::parse(&bytes).is_ok()
            }
        })
    })
}
