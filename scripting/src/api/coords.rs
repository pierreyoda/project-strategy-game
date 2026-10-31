//! Coordinates, as scripts write them: `#{q, r}`, axial.
//!
//! Geometry that depends on the map - neighbors, distance - lives on the view, which knows the map
//! it is a view of; what is here works on any map.

use rhai::{Engine, INT, Map};

use project_x_core::hex_map::coordinates::{CubeCoords, CubeCoordsScalar, OffsetCoords};

use super::ScriptResult;

/// How far from the origin a script's coordinates may go.
///
/// Far beyond any map, and far enough below `CubeCoordsScalar::MAX` that nothing computed from
/// two of them - a neighbor, a lap around the map - can overflow.
const BOUND: INT = 8192;

pub fn register(engine: &mut Engine) {
    engine
        .register_fn("axial", |q: INT, r: INT| -> ScriptResult<Map> {
            Ok(to_map(axial(q, r)?))
        })
        .register_fn("offset", |col: INT, row: INT| -> ScriptResult<Map> {
            let (col, row) = (scalar(col)?, scalar(row)?);
            Ok(to_map(OffsetCoords::new(col, row).into()))
        });
}

/// Coordinates as a script holds them.
pub fn to_map(coords: CubeCoords) -> Map {
    let mut map = Map::new();
    map.insert("q".into(), INT::from(coords.q()).into());
    map.insert("r".into(), INT::from(coords.r()).into());
    map
}

/// Coordinates from a script, or why they are not.
pub fn from_map(map: &Map) -> ScriptResult<CubeCoords> {
    let field = |name: &str| -> ScriptResult<INT> {
        map.get(name)
            .and_then(|value| value.as_int().ok())
            .ok_or_else(|| format!("coordinates need an integer `{name}`, got {map:?}").into())
    };
    axial(field("q")?, field("r")?)
}

fn axial(q: INT, r: INT) -> ScriptResult<CubeCoords> {
    Ok(CubeCoords::from_axial_coords(scalar(q)?, scalar(r)?))
}

fn scalar(value: INT) -> ScriptResult<CubeCoordsScalar> {
    if (-BOUND..=BOUND).contains(&value) {
        Ok(value as CubeCoordsScalar)
    } else {
        Err(format!("{value} is out of bounds for a coordinate").into())
    }
}

#[cfg(test)]
mod tests {
    use rhai::Map;

    use project_x_core::hex_map::coordinates::{CubeCoords, OffsetCoords};

    use super::{from_map, to_map};

    #[test]
    fn test_coordinates_survive_the_round_trip() {
        let coords: CubeCoords = OffsetCoords::new(7, 3).into();
        assert_eq!(from_map(&to_map(coords)).unwrap(), coords);
    }

    #[test]
    fn test_coordinates_out_of_bounds_or_malformed_are_refused() {
        let mut map = Map::new();
        map.insert("q".into(), 30_000_i64.into());
        map.insert("r".into(), 30_000_i64.into());
        assert!(from_map(&map).is_err(), "would overflow `s`");

        map.insert("q".into(), "east".into());
        assert!(from_map(&map).is_err());
        assert!(from_map(&Map::new()).is_err());
    }
}
