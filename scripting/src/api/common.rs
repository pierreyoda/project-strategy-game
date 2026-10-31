use std::collections::BTreeMap;

use project_x_core::simulation::resources::{Resource, ResourceQuantity};
use rhai::{INT, Map};

use crate::api::ScriptResult;

/// A resource by the name scripts know it by: the one the wire and the client use too
/// (`resource_name` in the `wasm_bridge` crate).
fn resource_named(name: &str) -> Option<Resource> {
    Some(match name {
        "credits" => Resource::Credits,
        "water" => Resource::Water,
        "food" => Resource::Food,
        "electricity" => Resource::Electricity,
        "metals" => Resource::Metals,
        "oil" => Resource::Oil,
        "uranium" => Resource::Uranium,
        _ => return None,
    })
}

/// A definition map, read strictly: a key it does not know is a typo, not something to ignore.
pub struct Fields<'a> {
    what: &'a str,
    map: &'a Map,
}

impl<'a> Fields<'a> {
    pub fn new(what: &'a str, map: &'a Map, known: &[&str]) -> ScriptResult<Self> {
        if let Some(unknown) = map.keys().find(|key| !known.contains(&key.as_str())) {
            return Err(format!("{what} has no `{unknown}`; it takes {}", known.join(", ")).into());
        }
        Ok(Self { what, map })
    }

    pub fn text(&self, key: &str) -> ScriptResult<String> {
        self.optional_text(key)?
            .ok_or_else(|| format!("{} needs `{key}`", self.what).into())
    }

    pub fn optional_text(&self, key: &str) -> ScriptResult<Option<String>> {
        let Some(value) = self.map.get(key) else {
            return Ok(None);
        };
        value
            .clone()
            .into_string()
            .map(Some)
            .map_err(|_| format!("{}'s `{key}` must be a string, got {value}", self.what).into())
    }

    /// A number, range-checked into what `core` stores it as.
    pub fn number<T: TryFrom<INT>>(&self, key: &str) -> ScriptResult<T> {
        let value = self
            .map
            .get(key)
            .ok_or_else(|| format!("{} needs `{key}`", self.what))?;
        value
            .as_int()
            .ok()
            .and_then(|number| T::try_from(number).ok())
            .ok_or_else(|| format!("{}'s `{key}` is out of range: {value}", self.what).into())
    }

    /// Quantities of resources, `#{credits: 60, metals: 15}`; none when the key is left out.
    pub fn quantities(&self, key: &str) -> ScriptResult<BTreeMap<Resource, ResourceQuantity>> {
        let Some(value) = self.map.get(key) else {
            return Ok(BTreeMap::new());
        };
        let quantities = value.read_lock::<Map>().ok_or_else(|| {
            format!(
                "{}'s `{key}` must map resources to quantities, got {value}",
                self.what
            )
        })?;
        quantities
            .iter()
            .map(|(name, quantity)| {
                let resource = resource_named(name).ok_or_else(|| {
                    format!(
                        "{}'s `{key}` names `{name}`, which is no resource",
                        self.what
                    )
                })?;
                let quantity = quantity
                    .as_int()
                    .ok()
                    .and_then(|quantity| ResourceQuantity::try_from(quantity).ok())
                    .ok_or_else(|| {
                        format!(
                            "{}'s `{key}.{name}` must be from 0 to {}, got {quantity}",
                            self.what,
                            ResourceQuantity::MAX
                        )
                    })?;
                Ok((resource, quantity))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rhai::{Array, Dynamic, EvalAltResult, Map};

    use project_x_core::simulation::resources::{Resource, ResourceQuantity};

    use super::Fields;
    use crate::api::ScriptResult;

    const UNIT: &str = "a unit template";
    const UNIT_KEYS: &[&str] = &["id", "type", "movement"];
    const BUILDING: &str = "a building template";
    const BUILDING_KEYS: &[&str] = &["id", "cost"];

    fn map<const N: usize>(entries: [(&str, Dynamic); N]) -> Map {
        entries
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect()
    }

    /// `#{cost: #{...}}`, with `entries` as the quantities.
    fn costing<const N: usize>(entries: [(&str, Dynamic); N]) -> Map {
        map([("cost", Dynamic::from_map(map(entries)))])
    }

    fn cost_of(definition: &Map) -> ScriptResult<BTreeMap<Resource, ResourceQuantity>> {
        Fields::new(BUILDING, definition, BUILDING_KEYS)?.quantities("cost")
    }

    /// The message a script is refused with.
    fn refusal<T>(result: ScriptResult<T>) -> String {
        match *result.err().expect("must be refused") {
            EvalAltResult::ErrorRuntime(message, _) => message.to_string(),
            other => panic!("refused with `{other}` rather than a message"),
        }
    }

    #[test]
    fn test_a_definition_may_leave_out_any_key_it_takes() {
        assert!(Fields::new(UNIT, &Map::new(), UNIT_KEYS).is_ok());
        assert!(Fields::new(UNIT, &map([("type", "scout".into())]), UNIT_KEYS).is_ok());
        let every_key = map([
            ("id", "scout".into()),
            ("type", "infantry".into()),
            ("movement", 2_i64.into()),
        ]);
        assert!(Fields::new(UNIT, &every_key, UNIT_KEYS).is_ok());
    }

    #[test]
    fn test_a_key_it_does_not_take_is_refused_with_the_keys_it_does() {
        let typo = map([("id", "scout".into()), ("movment", 2_i64.into())]);
        assert_eq!(
            refusal(Fields::new(UNIT, &typo, UNIT_KEYS)),
            "a unit template has no `movment`; it takes id, type, movement"
        );
        // matched exactly, not up to case
        assert!(Fields::new(UNIT, &map([("ID", "scout".into())]), UNIT_KEYS).is_err());
    }

    #[test]
    fn test_a_definition_that_takes_no_key_refuses_every_key() {
        assert!(Fields::new(UNIT, &Map::new(), &[]).is_ok());
        assert!(Fields::new(UNIT, &map([("id", "scout".into())]), &[]).is_err());
    }

    #[test]
    fn test_text_reads_a_string_and_is_required() {
        let definition = map([("id", "scout".into()), ("type", "".into())]);
        let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
        assert_eq!(fields.text("id").unwrap(), "scout");
        assert_eq!(
            fields.text("type").unwrap(),
            "",
            "an empty string is still text"
        );

        let empty = Map::new();
        let fields = Fields::new(UNIT, &empty, UNIT_KEYS).unwrap();
        assert_eq!(refusal(fields.text("id")), "a unit template needs `id`");
    }

    #[test]
    fn test_optional_text_reads_a_string_or_nothing() {
        let definition = map([("type", "infantry".into())]);
        let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
        assert_eq!(
            fields.optional_text("type").unwrap().as_deref(),
            Some("infantry")
        );
        assert_eq!(fields.optional_text("id").unwrap(), None);
    }

    #[test]
    fn test_text_that_is_no_string_is_refused_even_when_optional() {
        let definition = map([("id", 7_i64.into())]);
        let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
        assert_eq!(
            refusal(fields.text("id")),
            "a unit template's `id` must be a string, got 7"
        );
        assert_eq!(
            refusal(fields.optional_text("id")),
            "a unit template's `id` must be a string, got 7"
        );

        // a character is no string to Rhai, and a unit value is there rather than left out
        for value in [
            Dynamic::from_char('s'),
            Dynamic::UNIT,
            Dynamic::from_bool(true),
        ] {
            let definition = map([("id", value)]);
            let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
            assert!(fields.text("id").is_err());
            assert!(fields.optional_text("id").is_err());
        }
    }

    #[test]
    fn test_number_is_read_into_the_type_core_stores_it_as() {
        let definition = map([("movement", 255_i64.into())]);
        let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
        assert_eq!(fields.number::<u8>("movement").unwrap(), 255);
        assert_eq!(fields.number::<u16>("movement").unwrap(), 255);
        assert_eq!(fields.number::<i64>("movement").unwrap(), 255);

        let definition = map([("movement", 0_i64.into())]);
        let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
        assert_eq!(fields.number::<u8>("movement").unwrap(), 0);
    }

    #[test]
    fn test_number_out_of_its_type_range_is_refused() {
        for (value, shown) in [
            (256_i64, "256"),
            (-1, "-1"),
            (i64::MAX, "9223372036854775807"),
        ] {
            let definition = map([("movement", value.into())]);
            let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
            assert_eq!(
                refusal(fields.number::<u8>("movement")),
                format!("a unit template's `movement` is out of range: {shown}")
            );
        }
        let definition = map([("movement", (-1_i64).into())]);
        let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
        assert_eq!(
            fields.number::<i8>("movement").unwrap(),
            -1,
            "signed types take negatives"
        );
    }

    #[test]
    fn test_number_is_required_and_must_be_an_integer() {
        let empty = Map::new();
        let fields = Fields::new(UNIT, &empty, UNIT_KEYS).unwrap();
        assert_eq!(
            refusal(fields.number::<u8>("movement")),
            "a unit template needs `movement`"
        );

        // not even a float with nothing after the point, nor digits in a string
        for value in [Dynamic::from_float(2.0), "2".into(), Dynamic::UNIT] {
            let definition = map([("movement", value)]);
            let fields = Fields::new(UNIT, &definition, UNIT_KEYS).unwrap();
            assert!(fields.number::<u8>("movement").is_err());
        }
    }

    #[test]
    fn test_quantities_left_out_or_empty_are_none() {
        assert_eq!(cost_of(&Map::new()).unwrap(), BTreeMap::new());
        assert_eq!(cost_of(&costing([])).unwrap(), BTreeMap::new());
    }

    #[test]
    fn test_quantities_name_every_resource_as_the_wire_does() {
        let definition = costing([
            ("credits", 1_i64.into()),
            ("water", 2_i64.into()),
            ("food", 3_i64.into()),
            ("electricity", 4_i64.into()),
            ("metals", 5_i64.into()),
            ("oil", 6_i64.into()),
            ("uranium", 7_i64.into()),
        ]);
        assert_eq!(
            cost_of(&definition).unwrap(),
            BTreeMap::from([
                (Resource::Credits, 1),
                (Resource::Water, 2),
                (Resource::Food, 3),
                (Resource::Electricity, 4),
                (Resource::Metals, 5),
                (Resource::Oil, 6),
                (Resource::Uranium, 7),
            ])
        );
    }

    #[test]
    fn test_quantities_naming_no_resource_are_refused() {
        assert_eq!(
            refusal(cost_of(&costing([("gold", 5_i64.into())]))),
            "a building template's `cost` names `gold`, which is no resource"
        );
        // matched exactly, not up to case
        assert!(cost_of(&costing([("Credits", 5_i64.into())])).is_err());
    }

    #[test]
    fn test_quantities_are_bounded_by_what_a_resource_quantity_holds() {
        let bounds = costing([("credits", 0_i64.into()), ("metals", 65_535_i64.into())]);
        assert_eq!(
            cost_of(&bounds).unwrap(),
            BTreeMap::from([
                (Resource::Credits, 0),
                (Resource::Metals, ResourceQuantity::MAX)
            ])
        );

        for (value, shown) in [(65_536_i64, "65536"), (-1, "-1")] {
            assert_eq!(
                refusal(cost_of(&costing([("metals", value.into())]))),
                format!("a building template's `cost.metals` must be from 0 to 65535, got {shown}")
            );
        }
    }

    #[test]
    fn test_quantities_must_be_integers() {
        for value in [Dynamic::from_float(15.0), "15".into(), Dynamic::UNIT] {
            assert!(cost_of(&costing([("metals", value)])).is_err());
        }
    }

    #[test]
    fn test_quantities_must_be_a_map() {
        assert_eq!(
            refusal(cost_of(&map([("cost", 60_i64.into())]))),
            "a building template's `cost` must map resources to quantities, got 60"
        );
        let listed: Array = vec!["credits".into(), 60_i64.into()];
        for value in [Dynamic::from_array(listed), "credits".into()] {
            assert!(cost_of(&map([("cost", value)])).is_err());
        }
    }

    #[test]
    fn test_one_bad_quantity_refuses_them_all() {
        let definition = costing([("credits", 60_i64.into()), ("metals", (-15_i64).into())]);
        assert!(cost_of(&definition).is_err());
    }
}
