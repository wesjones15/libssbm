//! Effect dispatch reimplemented from efasync.c/efalt.c/efsync.c and Samus
//! article accessory callbacks. Numeric IDs identify disc objects, not assets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Particle(u32),
    Model(u32),
    NoOp,
    CameraShake(u32),
    Unknown,
}
pub fn resolve(id: u32) -> Target {
    use Target::*;
    match id {
        0..=591 => Particle(id),
        1000 => Model(10),
        1001 => Particle(12),
        1002 => Particle(20),
        1004 => Model(8),
        1011 => Particle(11),
        1015 => Model(18),
        1016 => Model(19),
        1021 => Model(3),
        1022 => Particle(263),
        1023 => Model(5),
        1026 => Particle(89),
        1027 => Particle(94),
        1028 => Model(24),
        1030 => Model(4),
        1031 => Particle(60),
        1036 => Particle(25),
        1038 => Particle(67),
        1041 => Particle(75),
        1060 => Model(2),
        1067 => Model(25), // efasync.c 0x42B: standard blast-zone KO
        1068 => Model(26), // alternate KO presentation
        1086 => Model(36),
        1151 => Particle(2004),
        1152 => Particle(2002),
        1153 => Particle(2003),
        1154 => Model(2000),
        1155 => Particle(2007),
        1156 => Particle(2011),
        1157 => Particle(2014),
        1158 => Model(2001),
        1159 => Model(2002),
        // ftCo_8009F834 routes this to Camera_RequestQuake, before efsync.
        1299 => CameraShake(2),
        1301 => Particle(4),
        _ => Unknown,
    }
}
pub fn article_cues(
    kind: u32,
    state: u32,
    previous: u32,
    frame: u32,
    full_charge: bool,
) -> Vec<u32> {
    let mut cues = Vec::new();
    if previous == u32::MAX {
        if kind == 2 {
            cues.push(if state == 0 { 1157 } else { 1156 });
        }
        return cues;
    }
    // it_802B5EDC: full charge accessories pulse on the 0,3,6,... clock.
    if kind == 1 && full_charge && frame >= previous {
        for f in previous..frame.min(previous.saturating_add(16)) {
            if f % 3 == 0 {
                cues.extend([1152, 1153]);
            }
        }
    }
    cues
}
