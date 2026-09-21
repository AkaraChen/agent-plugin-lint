//! JSON syntax is deliberately checked separately from `Value` conversion.
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Value, value::RawValue};
use std::collections::BTreeSet;
use std::fmt;

pub(crate) enum Parsed {
    Value { value: Value, duplicate_keys: bool },
    Syntax,
    Representation,
}

pub(crate) fn parse(source: &str) -> Parsed {
    if serde_json::from_str::<Box<RawValue>>(source).is_err() {
        return Parsed::Syntax;
    }
    let value = match serde_json::from_str::<Value>(source) {
        Ok(value) => value,
        Err(_) => return Parsed::Representation,
    };
    let mut duplicate_keys = false;
    let mut deserializer = serde_json::Deserializer::from_str(source);
    DuplicateSeed {
        found: &mut duplicate_keys,
    }
    .deserialize(&mut deserializer)
    .expect("Value-compatible JSON remains traversable");
    Parsed::Value {
        value,
        duplicate_keys,
    }
}

struct DuplicateSeed<'a> {
    found: &'a mut bool,
}
impl<'de> DeserializeSeed<'de> for DuplicateSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(DuplicateVisitor { found: self.found })
    }
}
struct DuplicateVisitor<'a> {
    found: &'a mut bool,
}
impl<'de> Visitor<'de> for DuplicateVisitor<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_string<E: serde::de::Error>(self, _: String) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq
            .next_element_seed(DuplicateSeed { found: self.found })?
            .is_some()
        {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut names = BTreeSet::new();
        while let Some(name) = map.next_key::<String>()? {
            if !names.insert(name) {
                *self.found = true;
            }
            map.next_value_seed(DuplicateSeed { found: self.found })?;
        }
        Ok(())
    }
}
