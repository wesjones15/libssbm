//! Shared hurt/fall/down presentation flow. ftCo_DamageFall, DownBound and
//! DownStand supply animation-completion transitions. The host signals damage;
//! RoR2 supplies health/knockback instead of Melee percent and hitstun rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    GroundHit,
    AirHit,
    Fall,
    Bound,
    Stand,
}
impl Stage {
    pub fn clip(self) -> &'static str {
        match self {
            Self::GroundHit => "DamageN2",
            Self::AirHit => "DamageFlyN",
            Self::Fall => "DamageFall",
            Self::Bound => "DownBoundU",
            Self::Stand => "DownStandU",
        }
    }
    pub fn after(self) -> Option<Self> {
        match self {
            Self::GroundHit | Self::Stand => None,
            Self::AirHit => Some(Self::Fall),
            Self::Fall => Some(Self::Fall),
            Self::Bound => Some(Self::Stand),
        }
    }
}
