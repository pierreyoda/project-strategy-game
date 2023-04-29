//! Central Pseudo-Random Number Generator module.

pub struct RandomGenerator {}

pub struct TestingHarnessRandomGenerator {}

pub trait CoreRandom {
    fn random_maxed_value_u32(&mut self, _max: u32) {}
}
