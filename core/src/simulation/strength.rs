//! A military unit's strength, in companies.
//!
//! Each company is in one of three states:
//! - **active**: it fights;
//! - **suppressed**: pinned down, disorganized - it does not fight, but it recovers at the end of the
//!   turn, as its template's `Recovery` says;
//! - **lost**: gone, until replaced.
//!
//! A unit with every remaining company suppressed is **broken**: it cannot hold its ground. One
//! with every company lost is **destroyed**. What breaks or destroys a unit is for combat to decide;
//! this only keeps the count.

/// The most companies a unit can have.
pub const MAX_UNIT_COMPANIES: u8 = 8;

/// How many of a unit's suppressed companies recover at the end of a turn.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Recovery {
    /// Every one of them.
    #[default]
    All,
    /// Up to this many.
    UpTo(u8),
}

/// What a blow does to a unit, in companies.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Damage {
    pub lost: u8,
    pub suppressed: u8,
}

/// Where a unit stands after taking damage.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StrengthOutcome {
    /// Some of its companies can still fight.
    Holding,
    /// Every company it has left is suppressed.
    Broken,
    /// Every company is lost.
    Destroyed,
}

/// A unit's companies, by state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Strength {
    companies: u8,
    suppressed: u8,
    lost: u8,
}

impl Strength {
    /// A unit at full strength: every company active.
    pub fn full(companies: u8) -> Self {
        assert!(
            (1..=MAX_UNIT_COMPANIES).contains(&companies),
            "a unit has from 1 to {MAX_UNIT_COMPANIES} companies, not {companies}"
        );
        Self {
            companies,
            suppressed: 0,
            lost: 0,
        }
    }

    /// Every company, whatever its state.
    pub fn companies(&self) -> u8 {
        self.companies
    }

    pub fn suppressed(&self) -> u8 {
        self.suppressed
    }

    pub fn lost(&self) -> u8 {
        self.lost
    }

    /// The companies that fight: neither suppressed nor lost.
    pub fn active(&self) -> u8 {
        self.companies - self.suppressed - self.lost
    }

    /// The companies not lost.
    pub fn remaining(&self) -> u8 {
        self.companies - self.lost
    }

    pub fn is_broken(&self) -> bool {
        self.remaining() > 0 && self.active() == 0
    }

    pub fn is_destroyed(&self) -> bool {
        self.remaining() == 0
    }

    pub fn outcome(&self) -> StrengthOutcome {
        if self.is_destroyed() {
            StrengthOutcome::Destroyed
        } else if self.is_broken() {
            StrengthOutcome::Broken
        } else {
            StrengthOutcome::Holding
        }
    }

    /// Take a blow: the losses first, off the active companies before the suppressed ones; then
    /// the suppression, of the active companies left. Suppression past them is lost companies
    /// instead: a unit that is hit when it is already broken loses ground for good.
    pub fn take(&mut self, damage: Damage) -> StrengthOutcome {
        let lost_active = damage.lost.min(self.active());
        let lost_suppressed = (damage.lost - lost_active).min(self.suppressed);
        self.lost += lost_active + lost_suppressed;
        self.suppressed -= lost_suppressed;

        let suppressed = damage.suppressed.min(self.active());
        self.suppressed += suppressed;
        let overflow = (damage.suppressed - suppressed).min(self.suppressed);
        self.suppressed -= overflow;
        self.lost += overflow;
        self.outcome()
    }

    /// Bring suppressed companies back into the fight. Returns how many recovered.
    pub fn recover(&mut self, recovery: Recovery) -> u8 {
        let recovered = match recovery {
            Recovery::All => self.suppressed,
            Recovery::UpTo(companies) => companies.min(self.suppressed),
        };
        self.suppressed -= recovered;
        recovered
    }
}

#[cfg(test)]
mod tests {
    use super::{Damage, MAX_UNIT_COMPANIES, Recovery, Strength, StrengthOutcome};

    fn damage(lost: u8, suppressed: u8) -> Damage {
        Damage { lost, suppressed }
    }

    #[test]
    fn test_a_full_unit_has_every_company_active() {
        let strength = Strength::full(4);
        assert_eq!(strength.companies(), 4);
        assert_eq!(strength.active(), 4);
        assert_eq!(strength.remaining(), 4);
        assert_eq!(strength.outcome(), StrengthOutcome::Holding);
    }

    #[test]
    #[should_panic(expected = "from 1 to 8 companies")]
    fn test_a_unit_has_at_least_one_company() {
        Strength::full(0);
    }

    #[test]
    #[should_panic(expected = "from 1 to 8 companies")]
    fn test_a_unit_has_at_most_the_cap() {
        Strength::full(MAX_UNIT_COMPANIES + 1);
    }

    #[test]
    fn test_losses_come_off_the_active_companies_first() {
        let mut strength = Strength::full(4);
        strength.take(damage(0, 2));
        assert_eq!(strength.take(damage(1, 0)), StrengthOutcome::Holding);
        assert_eq!((strength.active(), strength.suppressed()), (1, 2));
        // with no active company left, the suppressed ones go
        assert_eq!(strength.take(damage(2, 0)), StrengthOutcome::Broken);
        assert_eq!((strength.active(), strength.suppressed()), (0, 1));
        assert_eq!(strength.lost(), 3);
    }

    #[test]
    fn test_a_unit_with_every_remaining_company_suppressed_is_broken() {
        let mut strength = Strength::full(3);
        assert_eq!(strength.take(damage(1, 2)), StrengthOutcome::Broken);
        assert_eq!(
            (strength.active(), strength.suppressed(), strength.lost()),
            (0, 2, 1)
        );
    }

    #[test]
    fn test_suppression_past_the_active_companies_is_lost_companies() {
        let mut strength = Strength::full(4);
        assert_eq!(strength.take(damage(0, 6)), StrengthOutcome::Broken);
        assert_eq!((strength.suppressed(), strength.lost()), (2, 2));
        assert_eq!(strength.take(damage(0, 5)), StrengthOutcome::Destroyed);
        assert_eq!(strength.lost(), 4);
    }

    #[test]
    fn test_a_unit_with_every_company_lost_is_destroyed() {
        let mut strength = Strength::full(2);
        assert_eq!(strength.take(damage(200, 200)), StrengthOutcome::Destroyed);
        assert_eq!((strength.remaining(), strength.suppressed()), (0, 0));
    }

    #[test]
    fn test_recovery_brings_suppressed_companies_back() {
        let mut strength = Strength::full(5);
        strength.take(damage(1, 3));
        assert_eq!(strength.recover(Recovery::UpTo(2)), 2);
        assert_eq!((strength.active(), strength.suppressed()), (3, 1));
        assert_eq!(strength.recover(Recovery::UpTo(2)), 1);
        strength.take(damage(0, 4));
        assert_eq!(strength.recover(Recovery::All), 4);
        // lost companies stay lost
        assert_eq!((strength.active(), strength.lost()), (4, 1));
    }
}
