//! The survey of a whole generated world: the planet's terrain, then its settlement - the nations
//! and the deposits they start with.
//!
//! The two halves weigh the same in the world's score, so that the terrain's longer list of
//! criteria does not drown the settlement's. A few criteria are **mandatory**: a world missing one
//! fails outright, whatever its score - two continents too close, land on a closed pole, a nation
//! left out or short of its guarantees, a deposit on ice or under a capital. `view_survey` prints both halves over many seeds; the tests
//! hold the default settings to every mandatory criterion.

use std::fmt;

use crate::{
    deposits::{self, DepositsGeneratorSettings, DepositsSurvey, GeneratedDeposits},
    map::{
        Criterion, MapGenerator, MapGeneratorSeedScalar, MapGeneratorSettings, PlanetSurface,
        PlanetSurvey, Target, Unit, score,
    },
    nations::{GeneratedNations, NationsGeneratorSettings},
};

/// Everything it takes to generate a world, one seed for all three generators.
#[derive(Clone, Debug)]
pub struct WorldSettings {
    pub map: MapGeneratorSettings,
    pub nations: NationsGeneratorSettings,
    pub deposits: DepositsGeneratorSettings,
}

impl WorldSettings {
    /// The default settings of every generator.
    pub fn new(seed: MapGeneratorSeedScalar, columns: i16, rows: i16, nations: u16) -> Self {
        Self {
            map: MapGeneratorSettings::new(seed, columns, rows),
            nations: NationsGeneratorSettings::new(u64::from(seed), nations),
            deposits: DepositsGeneratorSettings::new(u64::from(seed)),
        }
    }

    /// The same settings, for another seed.
    pub fn with_seed(self, seed: MapGeneratorSeedScalar) -> Self {
        Self {
            map: self.map.with_seed(seed),
            nations: self.nations.with_seed(u64::from(seed)),
            deposits: self.deposits.with_seed(u64::from(seed)),
        }
    }

    /// The planet's surface, classified.
    pub fn generate_surface(&self) -> PlanetSurface {
        let map = MapGenerator::from_settings(self.map.clone()).generate(|_| {});
        PlanetSurface::classify(&map, &self.map)
    }

    /// The nations and deposits of a surface generated with these settings.
    pub fn settle(&self, surface: &PlanetSurface) -> (GeneratedNations, GeneratedDeposits) {
        deposits::settle(surface, &self.nations, &self.deposits)
    }
}

/// Everything the survey measured.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldSurvey {
    pub planet: PlanetSurvey,
    /// Nations placed, over the nations asked for.
    pub nations_placed: f64,
    pub deposits: DepositsSurvey,
}

impl WorldSurvey {
    /// The world generated with the given settings: its surface, nations and deposits.
    pub fn of(
        surface: &PlanetSurface,
        nations: &GeneratedNations,
        deposits: &GeneratedDeposits,
        settings: &WorldSettings,
    ) -> Self {
        Self::of_settled(
            PlanetSurvey::of(surface, settings.map.open_poles()),
            surface,
            nations,
            deposits,
            &settings.deposits,
        )
    }

    /// With the planet already surveyed: for settling the same planet over and over, as when
    /// tuning the nations and the deposits.
    pub fn of_settled(
        planet: PlanetSurvey,
        surface: &PlanetSurface,
        nations: &GeneratedNations,
        deposits: &GeneratedDeposits,
        deposits_settings: &DepositsGeneratorSettings,
    ) -> Self {
        let placed = nations.nations().len();
        let asked = placed + usize::from(nations.report().siting.unplaced);
        Self {
            planet,
            nations_placed: if asked == 0 {
                f64::NAN
            } else {
                placed as f64 / asked as f64
            },
            deposits: DepositsSurvey::of(surface, nations, deposits, deposits_settings),
        }
    }

    /// Generate the world and survey it.
    pub fn generate(settings: &WorldSettings) -> Self {
        let surface = settings.generate_surface();
        let (nations, deposits) = settings.settle(&surface);
        Self::of(&surface, &nations, &deposits, settings)
    }

    /// The terrain half: the planet's own criteria.
    pub fn terrain_criteria(&self) -> Vec<Criterion> {
        self.planet.criteria()
    }

    /// The settlement half: the nations', then the deposits'.
    pub fn settlement_criteria(&self) -> Vec<Criterion> {
        let mut criteria = vec![Criterion {
            id: "N1",
            name: "nations placed",
            value: self.nations_placed,
            unit: Unit::Share,
            target: Target::AtLeast(1.0),
            mandatory: true,
        }];
        criteria.extend(self.deposits.criteria());
        criteria
    }

    pub fn criteria(&self) -> Vec<Criterion> {
        let mut criteria = self.terrain_criteria();
        criteria.extend(self.settlement_criteria());
        criteria
    }

    /// The mandatory criteria the world misses: a criterion with nothing to measure misses too.
    pub fn mandatory_misses(&self) -> Vec<Criterion> {
        self.criteria()
            .into_iter()
            .filter(|criterion| criterion.mandatory && criterion.met() != Some(true))
            .collect()
    }

    pub fn meets_every_mandatory_criterion(&self) -> bool {
        self.mandatory_misses().is_empty()
    }

    /// From 0.0 to 1.0: the share of the terrain criteria met and the share of the settlement
    /// criteria met, averaged. Judges nothing mandatory: see `meets_every_mandatory_criterion`.
    pub fn score(&self) -> f64 {
        (share_met(&self.terrain_criteria()) + share_met(&self.settlement_criteria())) / 2.0
    }
}

/// Criteria met over criteria judged, 1.0 when none were.
pub fn share_met(criteria: &[Criterion]) -> f64 {
    let (met, judged) = score(criteria);
    if judged == 0 {
        1.0
    } else {
        met as f64 / judged as f64
    }
}

impl fmt::Display for WorldSurvey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "terrain")?;
        for criterion in self.terrain_criteria() {
            writeln!(f, "{criterion}")?;
        }
        writeln!(f, "settlement")?;
        for criterion in self.settlement_criteria() {
            writeln!(f, "{criterion}")?;
        }
        let misses: Vec<&str> = self
            .mandatory_misses()
            .iter()
            .map(|criterion| criterion.id)
            .collect();
        write!(f, "score {:.0}%, ", self.score() * 100.0)?;
        if misses.is_empty() {
            write!(f, "every mandatory criterion met")
        } else {
            write!(f, "mandatory criteria missed: {}", misses.join(", "))
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{WorldSettings, WorldSurvey, share_met};
    use crate::map::Criterion;

    /// The seeds `view_survey` surveys.
    pub(crate) fn survey_seeds() -> Vec<u32> {
        (0..32).map(|i| i * 7919 + 3).collect()
    }

    /// Surveys the default world over many seeds, criterion by criterion: 80 × 52, 8 nations.
    /// Run with:
    /// `cargo test -p project_x_procgen view_survey --release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn view_survey() {
        let seeds = survey_seeds();
        let surveys: Vec<WorldSurvey> = seeds
            .iter()
            .map(|&seed| WorldSurvey::generate(&WorldSettings::new(seed, 80, 52, 8)))
            .collect();
        println!("{} seeds, 80 x 52, 8 nations\n", seeds.len());
        println!("terrain");
        let terrain: Vec<Vec<Criterion>> =
            surveys.iter().map(WorldSurvey::terrain_criteria).collect();
        print!("{}", tabulate(&terrain));
        println!("\nsettlement");
        let settlement: Vec<Vec<Criterion>> = surveys
            .iter()
            .map(WorldSurvey::settlement_criteria)
            .collect();
        print!("{}", tabulate(&settlement));

        let mut scores: Vec<f64> = surveys.iter().map(WorldSurvey::score).collect();
        scores.sort_by(f64::total_cmp);
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let passing = surveys
            .iter()
            .filter(|survey| survey.meets_every_mandatory_criterion())
            .count();
        println!(
            "\nworld score: mean {:.1}%, min {:.0}% median {:.0}% max {:.0}%",
            mean * 100.0,
            scores[0] * 100.0,
            scores[scores.len() / 2] * 100.0,
            scores[scores.len() - 1] * 100.0,
        );
        println!(
            "every mandatory criterion met on {passing} of {} seeds",
            seeds.len()
        );
    }

    /// The criteria of many surveys, one line per criterion: how often it was met, and the spread
    /// of its values; then the spread of the share met per survey. `!` marks the mandatory ones.
    pub(crate) fn tabulate(surveys: &[Vec<Criterion>]) -> String {
        let mut text = String::new();
        for (position, criterion) in surveys[0].iter().enumerate() {
            let mut values: Vec<f64> = surveys
                .iter()
                .map(|criteria| criteria[position].value)
                .filter(|value| !value.is_nan())
                .collect();
            values.sort_by(f64::total_cmp);
            let met = surveys
                .iter()
                .filter(|criteria| criteria[position].met() == Some(true))
                .count();
            let (min, median, max) = if values.is_empty() {
                (f64::NAN, f64::NAN, f64::NAN)
            } else {
                (
                    values[0],
                    values[values.len() / 2],
                    values[values.len() - 1],
                )
            };
            let unit = criterion.unit;
            text.push_str(&format!(
                "{:>3}% met  {}{} {} ({}): min {} median {} max {}\n",
                met * 100 / surveys.len(),
                criterion.id,
                if criterion.mandatory { "!" } else { "" },
                criterion.name,
                unit.format_target(criterion.target),
                unit.format(min),
                unit.format(median),
                unit.format(max),
            ));
        }
        let mut shares: Vec<f64> = surveys.iter().map(|criteria| share_met(criteria)).collect();
        shares.sort_by(f64::total_cmp);
        text.push_str(&format!(
            "share met per seed: min {:.0}% median {:.0}% max {:.0}%\n",
            shares[0] * 100.0,
            shares[shares.len() / 2] * 100.0,
            shares[shares.len() - 1] * 100.0,
        ));
        text
    }

    #[test]
    fn test_world_survey_halves_weigh_the_same() {
        let survey = WorldSurvey::generate(&WorldSettings::new(3, 40, 26, 6));
        println!("{survey}");
        let terrain = share_met(&survey.terrain_criteria());
        let settlement = share_met(&survey.settlement_criteria());
        assert!((survey.score() - (terrain + settlement) / 2.0).abs() < 1e-12);
        assert_eq!(survey.settlement_criteria()[0].id, "N1");
        let mandatory: Vec<&str> = survey
            .criteria()
            .iter()
            .filter(|criterion| criterion.mandatory)
            .map(|criterion| criterion.id)
            .collect();
        assert_eq!(mandatory, ["A8", "D1", "D3", "N1", "E7", "F1", "F8"]);
    }

    #[test]
    fn test_world_survey_fails_a_world_missing_a_mandatory_criterion() {
        let mut survey = WorldSurvey::generate(&WorldSettings::new(3, 40, 26, 6));
        assert!(survey.meets_every_mandatory_criterion(), "{survey}");
        survey.nations_placed = 0.5;
        let misses: Vec<&str> = survey.mandatory_misses().iter().map(|c| c.id).collect();
        assert_eq!(misses, ["N1"]);
        assert!(
            survey
                .to_string()
                .ends_with("mandatory criteria missed: N1")
        );
        survey.nations_placed = f64::NAN;
        assert!(
            !survey.meets_every_mandatory_criterion(),
            "nothing to measure is a miss"
        );
    }

    /// The defaults hold every mandatory criterion, on more seeds than `view_survey` shows and at
    /// a smaller size too.
    #[test]
    fn test_default_world_meets_every_mandatory_criterion() {
        for (index, seed) in (0..6).map(|i| i * 15_485_863 + 101).enumerate() {
            let (columns, rows) = if index % 2 == 0 { (80, 52) } else { (40, 26) };
            let survey = WorldSurvey::generate(&WorldSettings::new(seed, columns, rows, 8));
            assert!(
                survey.meets_every_mandatory_criterion(),
                "seed {seed} at {columns} x {rows}:\n{survey}"
            );
        }
    }
}
