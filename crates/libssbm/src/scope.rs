//! The Super Scope as a fighter's held weapon: its ammo, charge levels and the shot's cost.
//! References: it/kinds/itsscope.c (the item: ammo, charge level, cost), ftCommon/ftCo_ItemScope{Start,Fire,Rapid}.c
//! (the fighter's states: charge while the button is held, fire on release, or the rapid taps of a quick press),
//! it/kinds/itsscopebeam.c (the beam).
//!
//! Tuning is the disc's: `itSScopeAttributes` and `ScopeBeamAttrs` in ItCo.dat and two words of
//! `ftCommonData` in PlCo.dat (read by libssbm-assets). Host adaptations are marked where they occur.
use glam::Vec3;

/// Charge levels a shot can have (0 is the uncharged tap).
pub const LEVELS: usize = 10;

/// The disc's Super Scope tuning.
#[derive(Clone, Debug)]
pub struct ScopeParams {
    /// A full magazine (itSScopeAttributes.x0).
    pub ammo: u32,
    /// A hold of this many ticks or fewer is a tap, level 0 (x4).
    pub charge_floor: u32,
    /// Level 9 is reached after this many times eight ticks (x8).
    pub charge_levels: u32,
    /// Ammo each level costs (xC).
    pub costs: [u32; LEVELS],
    /// Where the beam leaves, in the item's own space (x34).
    pub muzzle: Vec3,
    /// Ticks the fighter holds the charge before it fires on its own (ftCommonData.x5B8).
    pub hold_max: f32,
    /// Taps in a rapid burst (ftCommonData.x5BC).
    pub rapid_shots: u32,
    /// Host adaptation: shots of ammo the magazine refills a second once the scope has been idle a moment
    /// (0: it never refills, as in the source).
    pub refill_per_second: f32,
    /// Host adaptation: the magazine never spends. The runtime loader turns this on; fixtures keep the
    /// source's finite magazine so the ammo rules stay testable.
    pub infinite: bool,
}

impl ScopeParams {
    /// Synthetic tuning for headless tests; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            ammo: 48,
            charge_floor: 5,
            charge_levels: 8,
            costs: [1, 2, 3, 4, 6, 8, 10, 12, 14, 16],
            muzzle: Vec3::new(0., 5.7, 0.),
            hold_max: 90.,
            rapid_shots: 10,
            refill_per_second: 0.,
            infinite: false,
        }
    }

    /// `it_80291CF4`: the charge level a hold of `held` ticks reaches.
    pub fn level(&self, held: u32) -> u32 {
        if held <= self.charge_floor {
            0
        } else if held < self.charge_levels * 8 {
            held / 8
        } else {
            9
        }
    }

    /// `it_80291D38`: what a shot of `level` costs.
    pub fn cost(&self, level: u32) -> u32 {
        self.costs[(level as usize).min(LEVELS - 1)]
    }

    /// `it_80291DAC`: the level a hold of `held` ticks fires with `ammo` left, or `None` when the magazine
    /// is empty. A magazine that cannot pay for the charge fires the highest level it can pay for, the
    /// source's search being that it takes a level whose cost the ammo covers exactly, or the next one
    /// up, which uses the last of it.
    pub fn affordable(&self, ammo: u32, held: u32) -> Option<u32> {
        if self.infinite {
            return Some(self.level(held));
        }
        if ammo == 0 {
            return None;
        }
        let mut level = self.level(held);
        if ammo >= self.cost(level) {
            return Some(level);
        }
        while level > 0 {
            let cost = self.cost(level);
            if ammo == cost {
                return Some(level);
            }
            if ammo > cost {
                return Some(level + 1);
            }
            level -= 1;
        }
        Some(0)
    }
}

/// Where the fighter is in using the scope (`ItemScopeStart`, `Fire`, `Rapid`, `End`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScopeStage {
    #[default]
    Start,
    Fire,
    Rapid,
    End,
}

/// A fighter's Super Scope: the disc's tuning and the state of its magazine and use.
#[derive(Clone, Debug)]
pub struct Scope {
    pub params: ScopeParams,
    pub state: ScopeState,
}

/// The scope's state across ticks: its ammo, and the hold being built up.
#[derive(Clone, Debug, Default)]
pub struct ScopeState {
    /// Host lower-body jump-squat clock; independent of charge/fire timing.
    pub jump_squat: Option<u32>,
    /// Native button bit that started this use (`NEUTRAL_B` on RT, `UP_B` on LB). The
    /// hold, charge and release all read this so the Scope works on any skill slot.
    pub trigger: u32,
    /// Shots left in the magazine.
    pub ammo: u32,
    /// Ticks the button has been held in this use (`mv.co.x0` timer, 0x2340).
    pub held: u32,
    /// The button has been let go in this use (the flag at 0x2344).
    pub released: bool,
    /// The level of the shot being fired (`mv.co.common.x0`).
    pub level: Option<u32>,
    /// Rapid taps left (`mv.co.common.x0` in the rapid state).
    pub shots_left: u32,
    /// The Rapid clip's frame last tick, to see it come round (the disc marks the clip as looping, or the sim
    /// restarts it).
    pub last_frame: f32,
    /// Ticks since a shot was last fired (for the host's slow refill).
    pub idle: u32,
    /// Fractional ammo refilled so far.
    pub refill_carry: f32,
    /// The held-charge cue (SFX 0xFE, SFX_IT_SCORPE_TAME) has been started for this use.
    pub charge_sound: bool,
    /// Ticks since the Start clip ran out. The clip holds its last pose, but the Start script goes on: its
    /// loop spawns the ground dust (cue 0x3FF) every eight frames from frame 24 to 88.
    pub tail: u32,
}

impl ScopeState {
    pub fn new(params: &ScopeParams) -> Self {
        Self { ammo: params.ammo, ..Self::default() }
    }

    /// `it_80291F14`: pays for a shot of `level`; a magazine never goes below empty.
    pub fn spend(&mut self, params: &ScopeParams, level: u32) {
        if !params.infinite {
            self.ammo = self.ammo.saturating_sub(params.cost(level));
        }
        self.idle = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_is_the_hold_in_eights_between_the_floor_and_the_top() {
        let p = ScopeParams::fixture();
        assert_eq!(p.level(0), 0);
        assert_eq!(p.level(5), 0, "a hold at the floor is still a tap");
        assert_eq!(p.level(6), 0);
        assert_eq!(p.level(8), 1);
        assert_eq!(p.level(23), 2);
        assert_eq!(p.level(63), 7);
        assert_eq!(p.level(64), 9, "past eight times eight ticks is the top level");
        assert_eq!(p.level(1000), 9);
    }

    #[test]
    fn a_full_magazine_fires_the_charged_level_and_pays_its_cost() {
        let p = ScopeParams::fixture();
        assert_eq!(p.affordable(48, 30), Some(3));
        let mut s = ScopeState::new(&p);
        s.spend(&p, 3);
        assert_eq!(s.ammo, 44);
    }

    #[test]
    fn a_low_magazine_fires_what_it_can_pay_for_and_uses_the_last_of_it() {
        let p = ScopeParams::fixture();
        // 5 left, a level-5 hold (costs 8): level 4 costs 6, still more than 5 -> level 3 costs 4, 5 > 4 -> level 4.
        assert_eq!(p.affordable(5, 40), Some(4));
        let mut s = ScopeState { ammo: 5, ..Default::default() };
        s.spend(&p, 4);
        assert_eq!(s.ammo, 0, "the shot takes all that is left");
        // Exactly the cost of a lower level.
        assert_eq!(p.affordable(4, 40), Some(3));
        assert_eq!(p.affordable(0, 40), None);
    }
}
