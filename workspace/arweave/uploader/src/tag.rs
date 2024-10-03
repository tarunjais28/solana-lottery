use {
    anyhow::Result,
    avro_rs::{to_avro_datum, Schema},
    lazy_static::lazy_static,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

const SCHEMA_STR: &str = r##"{
    "type": "array",
    "items": {
        "type": "record",
        "name": "Tag",
        "fields": [
            { "name": "name", "type": "string" },
            { "name": "value", "type": "string" }
        ]
    }
}"##;

lazy_static! {
    pub static ref TAGS_SCHEMA: Schema = Schema::parse_str(SCHEMA_STR).unwrap();
}

pub trait AvroEncode {
    fn encode(&self) -> Result<Vec<u8>>;
}

impl AvroEncode for Vec<Tag> {
    fn encode(&self) -> Result<Vec<u8>> {
        let v = avro_rs::to_value(self)?;
        let bytes = to_avro_datum(&TAGS_SCHEMA, v)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_encode() {
        use super::*;
        let tags = vec![Tag {
            name: "name".to_string(),
            value: "value".to_string(),
        }];
        let encoded = tags.encode().unwrap();
        let expected = &[2u8, 8, 110, 97, 109, 101, 10, 118, 97, 108, 117, 101, 0];
        assert_eq!(encoded, expected);
    }
}
