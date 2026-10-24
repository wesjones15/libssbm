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
        1005 => Particle(80), // efasync.c 0x3ED: ice hit (a second generator, 84, is its companion)
        1029 => Particle(44), // efasync.c 0x405
        1046 => Particle(406), // efasync.c 0x416: dark hit
        1053 => Model(15),    // efasync.c 0x41D
        1145 => NoOp,         // efalt.c 0x479 spawns generator 1010, which no bank holds
        1004 => Model(8),
        1007 => Particle(66), // efasync.c 0x3EF/0x3F0: ground dust
        1008 => Particle(66),
        1011 => Particle(11),
        1012 => Particle(72), // efasync.c 0x3F4: ground dust
        1013 => Model(16),
        1014 => Model(17),
        1015 => Model(18),
        1016 => Model(19),
        1017 => Model(20),
        1018 => Model(21),
        1019 => Model(22),
        1020 => Model(23),
        1021 => Model(3),
        1022 => Particle(263),
        1023 => Model(5),
        1024 => Particle(90), // efasync.c 0x400/0x401: ground dust
        1025 => Particle(90),
        1026 => Particle(89),
        1027 => Particle(94),
        1028 => Model(24),
        1030 => Model(4),
        1031 => Particle(60),
        1036 => Particle(25),
        1038 => Particle(67),
        1041 => Particle(75),
        1043 => Particle(55),
        1058 => Particle(91),
        1059 => Model(1),
        1060 => Model(2),
        1065 => Particle(206),
        1067 => Model(25), // efasync.c 0x42B: standard blast-zone KO
        1068 => Model(26), // alternate KO presentation
        1072 => Particle(0x140), // efasync.c 0x430: Super Scope muzzle flash (generator 0x140)
        1080 => Model(34),
        1086 => Model(36),
        1087 => Particle(202), // efasync.c 0x43F: rebirth ring
        1094 => Particle(27),  // efasync.c 0x446: Captain Falcon Punch fist flame
        1151 => Particle(2004),
        1152 => Particle(2002),
        1153 => Particle(2003),
        1154 => Model(2000),
        1155 => Particle(2007),
        1156 => Particle(2011),
        1157 => Particle(2014),
        1158 => Model(2001),
        1159 => Model(2002),
        // Fox's own effects (efalt.c 0x488..0x48D): the Reflector shine
        // (loop/start/reflect bubble), Fire Fox charge flame, Fire Fox launch
        // flame and the Illusion blue streak.
        1160 => Model(2000),
        1161 => Model(2001),
        1162 => Model(2002),
        1163 => Model(2003),
        1164 => Model(2004),
        1165 => Particle(3008),
        // Captain Falcon's own effects (efalt.c 0x48F..0x493): Falcon Punch
        // wind, Falcon Kick flame and the Raptor Boost streaks.
        1167 => Model(2000),
        1168 => Model(2002),
        1169 => Model(2004),
        1170 => Model(2003),
        1171 => Model(2005),
        // Link's own effects (efsync.c 0x4BB/0x4BC): grounded and aerial
        // Spin Attack slashes.
        1211 => Model(2000),
        1212 => Model(2001),
        // Luigi's own effects (efsync.c 0x509/0x50A): the Cyclone spin and the
        // Green Missile launch flash.
        1289 => Model(2001),
        1290 => Particle(95),
        // Donkey Kong's own effects (efsync.c 0x4C6..0x4CC): one model each from his
        // effect archive (ids 8000..8006), for the Giant Punch swing (1224 ground, 1225
        // air), Spinning Kong (1226), the Hand Slap (1222, 1223, 1227, 1228).
        1222 => Model(2000),
        1223 => Model(2001),
        1224 => Model(2002),
        1225 => Model(2003),
        1226 => Model(2004),
        1227 => Model(2005),
        1228 => Model(2006),
        // Yoshi's own effects (efsync.c 0x4CD/0x4CE): the egg model (9000) and its
        // generator (9000), plus the shell bits of 0x4CF/0x4D0 (common models 30 and 28).
        1229 => Model(2000),
        1230 => Particle(9000),
        1231 => Model(30),
        1232 => Model(28),
        // Pikachu's own effects (efsync.c 0x4BD..0x4C5; EfPkData emitters
        // 7000..7009 = generator ids 0x1B58..) plus the common spark
        // (efasync.c 0x412 -> common generator 0x13) its move scripts emit.
        1042 => Particle(19),
        1213 => Particle(7000), // Thunder Jolt orb trail
        1214 => Particle(7004), // Skull Bash charge
        1215 => Particle(7005), // Skull Bash release
        1216 => Particle(7000), // Thunder strike on Pikachu
        1217 => Particle(7001),
        1218 => Particle(7002),
        1219 => Particle(588),  // efsync.c 0x4C3: Thunder cloud poof (common 0x24C)
        1220 => Particle(7003),
        1221 => Particle(7004),
        // ftCo_8009F834 routes this to Camera_RequestQuake, before efsync.
        1299 => CameraShake(2),
        1300 => Particle(3), // iteffect.c 0x514: item effect 3
        1301 => Particle(4),
        _ => Unknown,
    }
}
/// A second effect the source spawns with `id` in the same call (a `next` link of the
/// first): Falcon Punch's second flame and the child slash of Link's Spin Attack.
pub fn companion(id: u32) -> Option<Target> {
    Some(match id {
        1005 => Target::Particle(84),
        1167 => Target::Model(2001), // efalt.c 0x48F: 4000 and 4001
        1211 => Target::Model(2002), // efsync.c 0x4BB: 6000 and 6002
        1212 => Target::Model(2003), // efsync.c 0x4BC: 6001 and 6003
        _ => return None,
    })
}

/// Effects the source attaches to a joint (efLib_Create_Attach*, hsd_8039EFAC), which then
/// follow it; the rest are placed once at a position (Attach_Pos, generators).
pub fn attached(id: u32) -> bool {
    matches!(
        id,
        1026 | 1027 | 1060 | 1086 | 1151 | 1154 | 1159 | 1214 | 1215 | 1216 | 1301
            // Fox 0x488..0x48D, Captain 0x48F..0x493, Link 0x4BB/0x4BC, Donkey 0x4C6..0x4CB,
            // Yoshi 0x4CD, Luigi 0x509/0x50A. 0x48D (the Illusion streak) is a generator
            // attached to TopN, not a one-shot position effect.
            | 1160..=1165
            | 1167..=1171
            | 1211
            | 1212
            | 1222..=1227
            | 1229
            | 1289
            | 1290
    )
}

/// FtPart indices (ft/forward.h) the fighters' effects attach to.
pub mod part {
    pub const TOP_N: usize = 0;
    pub const TRANS_N: usize = 1;
    pub const HIP_N: usize = 4;
    pub const L_FOOT_JA: usize = 9;
    pub const R_FOOT_JA: usize = 14;
    pub const HEAD_N: usize = 34;
    pub const L_2ND_NA: usize = 24;
    pub const R_3RD_NA: usize = 44;
}

/// A cue for effect `id` on a common bone of the fighter, as `efSync_Spawn(id, gobj,
/// parts[bone].joint)` does.
pub fn part_cue(sim: &crate::sim::Sim, id: u32, bone: usize) -> crate::action::GfxCue {
    crate::action::GfxCue {
        id,
        bone,
        common: true,
        cycle: 0,
        destroy_on_change: false,
        offset: glam::Vec3::ZERO,
        param: 0.,
        clip: sim.motion.clip,
        frame: sim.motion.frame,
        origin: sim.position,
        facing: sim.facing,
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve, Target};

    #[test]
    fn the_scope_effects_are_common_particles() {
        // efasync.c 0x405: the empty-click smoke puff (generator 0x2C), reused for the held charge.
        assert_eq!(resolve(0x405), Target::Particle(44));
        // efasync.c 0x430: the shot's muzzle flash (generator 0x140).
        assert_eq!(resolve(0x430), Target::Particle(0x140));
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
