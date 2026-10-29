//! Deterministic random number generation for the game simulation.
//!
//! Determinism is not a nicety here: it is what makes a turn reproducible from a seed, which in
//! turn makes replays, `seed + command log` saves, and any kind of AI debugging possible at all.
//!
//! The generator is therefore **split**, never shared: every subsystem and every nation draws from
//! its own stream, so that an AI drawing one more number than it did last run cannot shift the
//! numbers the economy gets. A single shared stream would make every outcome depend on the exact
//! order and count of every draw in the turn.
//!
//! Drawing goes through `rand`: `GameRng` is an `Rng`, so ranges, floats, booleans and shuffles all
//! come from `rand::RngExt` and `rand::seq`.

use std::convert::Infallible;

use rand::{SeedableRng, TryRng, rngs::ChaCha8Rng};

use crate::simulation::ids::{EntityIndex, NationId};

/// Seed of a whole playthrough.
pub type GameSeed = u64;

/// What a stream is drawn for. Each purpose gets its own stream, so two purposes never line up.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RngPurpose {
    WorldGeneration,
    Economy,
    Logistics,
    Military,
    Population,
    /// One stream per nation, so that one AI's draws never shift another's.
    Ai(NationId),
}

impl RngPurpose {
    /// The ChaCha stream this purpose draws from: the purpose's number in the upper 32 bits, and
    /// the nation's index - for a per-nation purpose - in the lower 32.
    ///
    /// The numbers **must never change**: renumbering a purpose changes every number it draws, and
    /// with it every replay recorded before.
    fn stream(self) -> u64 {
        let (purpose, nation): (u32, u32) = match self {
            Self::WorldGeneration => (0, 0),
            Self::Economy => (1, 0),
            Self::Logistics => (2, 0),
            Self::Military => (3, 0),
            Self::Population => (4, 0),
            Self::Ai(nation) => (5, nation.index() as u32),
        };
        (u64::from(purpose) << 32) | u64::from(nation)
    }
}

/// A seeded generator, dedicated to one purpose.
///
/// ChaCha8 rather than `rand`'s `StdRng` or `SmallRng`: those two reserve the right to switch
/// algorithm from one release to the next, while ChaCha8 is a fixed algorithm whose output is the
/// same on every platform - the WASM build included. It also has 2^64 independent streams built
/// in, which is exactly what the split above needs. It is **not** used for its cryptographic
/// strength, and nothing here should ever want that.
///
/// Not `Clone`: `chacha20` deliberately leaves that out, since a cloned stream silently repeats
/// numbers already drawn.
#[derive(Debug)]
pub struct GameRng(ChaCha8Rng);

impl GameRng {
    /// The stream dedicated to `purpose`, in the playthrough seeded with `seed`.
    pub fn new(seed: GameSeed, purpose: RngPurpose) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        rng.set_stream(purpose.stream());
        Self(rng)
    }
}

/// Forwarded as is. Implementing `TryRng` is what makes `GameRng` an `Rng`.
impl TryRng for GameRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        self.0.try_next_u32()
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        self.0.try_next_u64()
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.0.try_fill_bytes(dst)
    }
}

#[cfg(test)]
mod tests {
    use rand::{Rng, RngExt};

    use crate::simulation::ids::{EntityIndex, NationId};

    use super::{GameRng, RngPurpose};

    fn draws(seed: u64, purpose: RngPurpose) -> Vec<u64> {
        let mut rng = GameRng::new(seed, purpose);
        (0..16).map(|_| rng.next_u64()).collect()
    }

    #[test]
    fn test_rng_is_reproducible_from_its_seed() {
        assert_eq!(
            draws(42, RngPurpose::Economy),
            draws(42, RngPurpose::Economy)
        );
        assert_ne!(
            draws(42, RngPurpose::Economy),
            draws(43, RngPurpose::Economy)
        );
    }

    #[test]
    fn test_rng_streams_are_independent() {
        assert_ne!(
            draws(7, RngPurpose::Economy),
            draws(7, RngPurpose::Military)
        );

        let ai = |index| RngPurpose::Ai(NationId::from_index(index));
        assert_ne!(draws(7, ai(0)), draws(7, ai(1)));
        // the first nation's AI must not be mistaken for a nation-wide purpose
        assert_ne!(draws(7, ai(0)), draws(7, RngPurpose::WorldGeneration));
    }

    /// The exact numbers a seed gives, frozen.
    ///
    /// If this fails after bumping `rand`, the bump changed the numbers every saved game and every
    /// replay was built on: it is a breaking change for players, not a test to update in passing.
    #[test]
    fn test_rng_output_is_frozen() {
        let mut rng = GameRng::new(42, RngPurpose::Economy);
        assert_eq!(rng.next_u64(), 10_517_904_778_964_690_192);
        assert_eq!(rng.random_range(0..1000), 65);
    }
}
