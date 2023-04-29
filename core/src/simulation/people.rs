//! Module (or, if it grows too bug, crate that will be extracted from `core`) for representing game entities
//! having human, alien or artificial (robots) consciousness; either at the individual level or as a "population aggregate" (from thousands to millions of people).

use std::hash::{Hash, Hasher};

pub mod leaders;
pub mod population;
