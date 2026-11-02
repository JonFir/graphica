use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Number, Value};
use std::fmt;
use yaml_rust2::{
    parser::{Event, Parser},
    scanner::TScalarStyle,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub pointer: String,
}

pub(crate) fn diag(
    code: &str,
    message: impl Into<String>,
    pointer: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        message: message.into(),
        pointer: pointer.into(),
    }
}

struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON value")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Number::from_f64(v)
                    .map(|n| UniqueValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-JSON number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(v.into())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(v)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<UniqueValue>()? {
                    items.push(item.0);
                }
                Ok(UniqueValue(Value::Array(items)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut items = Map::new();
                while let Some((key, value)) = map.next_entry::<String, UniqueValue>()? {
                    if items.insert(key.clone(), value.0).is_some() {
                        return Err(de::Error::custom(format!("duplicate mapping key: {key}")));
                    }
                }
                Ok(UniqueValue(Value::Object(items)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

fn ambiguous_plain(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "true"
            | "false"
            | "null"
            | "~"
            | "yes"
            | "no"
            | "on"
            | "off"
            | ".nan"
            | ".inf"
            | "+.inf"
            | "-.inf"
    ) {
        return true;
    }
    if s.is_empty() {
        return true;
    }
    let numeric = s.replace('_', "");
    if numeric.parse::<f64>().is_ok() {
        return true;
    }
    if numeric.starts_with("0x") || numeric.starts_with("0o") || numeric.starts_with("0b") {
        return true;
    }
    if numeric.contains(':')
        && numeric
            .split(':')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    {
        return true;
    }
    let bytes = s.as_bytes();
    bytes.len() >= 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn yaml_value(events: &[Event], index: &mut usize) -> Result<Value, String> {
    let event = events.get(*index).ok_or("missing YAML value")?;
    *index += 1;
    match event {
        Event::Scalar(text, style, _, _) => {
            if *style == TScalarStyle::Plain && ambiguous_plain(text) {
                return Err(format!("quote YAML scalar {:?} to make it a string", text));
            }
            Ok(Value::String(text.clone()))
        }
        Event::SequenceStart(..) => {
            let mut values = Vec::new();
            while !matches!(events.get(*index), Some(Event::SequenceEnd)) {
                values.push(yaml_value(events, index)?);
            }
            *index += 1;
            Ok(Value::Array(values))
        }
        Event::MappingStart(..) => {
            let mut values = Map::new();
            while !matches!(events.get(*index), Some(Event::MappingEnd)) {
                let key = yaml_value(events, index)?;
                let key = key
                    .as_str()
                    .ok_or("YAML mapping keys must be strings")?
                    .to_owned();
                if key == "<<" {
                    return Err("YAML merge keys are unsupported".into());
                }
                let value = yaml_value(events, index)?;
                if values.insert(key.clone(), value).is_some() {
                    return Err(format!("duplicate mapping key: {key}"));
                }
            }
            *index += 1;
            Ok(Value::Object(values))
        }
        _ => Err("invalid YAML structure".into()),
    }
}

pub fn load_diagram(text: &str, encoding: &str) -> Result<Value, Vec<Diagnostic>> {
    let fail = |message: String| vec![diag("invalid-encoding", message, "")];
    match encoding {
        "json" => {
            let mut deserializer = serde_json::Deserializer::from_str(text);
            let value = UniqueValue::deserialize(&mut deserializer)
                .map_err(|e| fail(e.to_string()))?
                .0;
            deserializer.end().map_err(|e| fail(e.to_string()))?;
            Ok(value)
        }
        "yaml" | "yml" => {
            let mut parser = Parser::new_from_str(text);
            let mut events = Vec::new();
            loop {
                let (event, _) = parser.next_token().map_err(|e| fail(e.to_string()))?;
                match &event {
                    Event::StreamEnd => break,
                    Event::Alias(_) => return Err(fail("YAML aliases are unsupported".into())),
                    Event::Scalar(_, _, anchor, tag)
                    | Event::SequenceStart(anchor, tag)
                    | Event::MappingStart(anchor, tag)
                        if *anchor != 0 || tag.is_some() =>
                    {
                        return Err(fail(
                            "YAML anchors and explicit tags are unsupported".into(),
                        ))
                    }
                    Event::StreamStart => continue,
                    _ => events.push(event),
                }
            }
            let mut index = 0;
            if !matches!(events.get(index), Some(Event::DocumentStart)) {
                return Err(fail("expected one YAML document".into()));
            }
            index += 1;
            let value = yaml_value(&events, &mut index).map_err(fail)?;
            if !matches!(events.get(index), Some(Event::DocumentEnd)) || index + 1 != events.len() {
                return Err(vec![diag(
                    "invalid-encoding",
                    "expected exactly one YAML document",
                    "",
                )]);
            }
            Ok(value)
        }
        _ => Err(fail(format!("unsupported encoding: {encoding}"))),
    }
}
