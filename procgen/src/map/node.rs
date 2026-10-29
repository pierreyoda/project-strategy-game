use std::rc::Rc;

use noise::{Cache, NoiseFn};

/// Output of a subgroup of noise functions.
///
/// Type-erased and cheap to clone, so that it can be shared by all the downstream subgroups using it.
///
/// Not thread-safe, since `Cache` is not: build one `PlanetGraph` per thread instead.
#[derive(Clone)]
pub struct NoiseNode(Rc<dyn NoiseFn<f64, 3>>);

impl NoiseNode {
    pub fn cached(module: impl NoiseFn<f64, 3> + 'static) -> Self {
        Self(Rc::new(Cache::new(module)))
    }
}

impl NoiseFn<f64, 3> for NoiseNode {
    fn get(&self, point: [f64; 3]) -> f64 {
        self.0.get(point)
    }
}
