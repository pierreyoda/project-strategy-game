//! Districts: the building slots a settlement lays out on the tiles around it.
//!
//! A district is a state of its tile, like its owner, rather than an entity of its own: it lives
//! on the tile's artificial layer, and only ever next to its settlement, so the ring around a
//! settlement reaches every district it has. Laying one out costs what the scenario's rules say,
//! paid at once, and takes their turns; once laid out, it is where a building that extracts nothing
//! may stand. An extractor stands on its deposit instead, district or not.

use std::fmt::Debug;

use super::{
    buildings::Works, economy::ConstructionCosts, ids::SettlementId, settlements::SettlementStage,
};

/// How far from its settlement a district is laid out: on the ring around it.
pub const DISTRICT_DISTANCE: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct District {
    settlement: SettlementId,
    /// The laying out under way; `None` once the district can be built on.
    works: Option<Works>,
}

impl District {
    /// A district just ordered: its works under way for `turns`.
    pub fn order(settlement: SettlementId, turns: u16) -> Self {
        Self {
            settlement,
            works: Some(Works::new(turns)),
        }
    }

    /// A district already laid out, as a scenario's setup or a test places one.
    pub fn laid_out(settlement: SettlementId) -> Self {
        Self {
            settlement,
            works: None,
        }
    }

    pub fn settlement(&self) -> SettlementId {
        self.settlement
    }

    pub fn works(&self) -> Option<&Works> {
        self.works.as_ref()
    }

    /// Whether its works are done: a building can be ordered on it.
    pub fn is_laid_out(&self) -> bool {
        self.works.is_none()
    }

    /// One turn of works. Returns whether they completed, which is when it can be built on.
    pub fn advance_works(&mut self) -> bool {
        let done = self.works.as_mut().is_some_and(Works::advance);
        if done {
            self.works = None;
        }
        done
    }
}

/// A settlement about to lay out a district, as the scenario's rules weigh it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DistrictSite {
    pub settlement: SettlementId,
    /// What its people amount to: a capital's size, not its role, which is `capital`.
    pub stage: SettlementStage,
    pub capital: bool,
    /// The districts it has, those still being laid out included.
    pub districts: u8,
    /// Its people, across its population groups.
    pub population: u32,
}

/// What laying out a district costs, and takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistrictTerms {
    pub cost: ConstructionCosts,
    /// At least 1.
    pub turns: u16,
}

/// The scenario's rule for what a settlement's next district costs: a script's function in a game,
/// a plain struct in a test.
///
/// It is handed the site and nothing else, so that it can neither read nor change the world: the
/// same site gives the same terms, and a playthrough replays from its seed and its commands.
pub trait DistrictRules: Debug {
    /// The terms of the site's next district, or why it cannot have one.
    fn terms(&self, site: &DistrictSite) -> Result<DistrictTerms, String>;
}

#[cfg(test)]
mod tests {
    use crate::simulation::ids::{EntityIndex, SettlementId};

    use super::District;

    #[test]
    fn test_an_ordered_district_can_be_built_on_once_its_works_are_done() {
        let mut district = District::order(SettlementId::from_index(0), 2);
        assert!(!district.is_laid_out());
        assert_eq!(district.works().map(|works| works.turns_left()), Some(2));

        assert!(!district.advance_works(), "one turn of two");
        assert!(district.advance_works(), "and the last");
        assert!(district.is_laid_out());
        assert_eq!(district.works(), None);
        assert!(!district.advance_works(), "nothing left to advance");
    }
}
