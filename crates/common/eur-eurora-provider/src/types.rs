//! Response types for gRPC providers.

use chrono::{DateTime, Utc};
use ferrous_llm_core::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::proto::chat::{ProtoImageSource, proto_image_source::ProtoSourceType};

use crate::proto::chat::{ProtoChatResponse, ProtoChatStreamResponse};

/// Convert proto Timestamp to DateTime<Utc>
pub fn proto_timestamp_to_datetime(timestamp: Option<prost_types::Timestamp>) -> DateTime<Utc> {
    timestamp
        .map(|ts| {
            DateTime::from_timestamp(ts.seconds, ts.nanos as u32).unwrap_or_else(|| Utc::now())
        })
        .unwrap_or_else(|| Utc::now())
}

/// Convert DateTime<Utc> to proto Timestamp
pub fn datetime_to_proto_timestamp(datetime: &DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: datetime.timestamp(),
        nanos: datetime.timestamp_subsec_nanos() as i32,
    }
}

/// Convert proto Struct to HashMap<String, serde_json::Value>
pub fn proto_struct_to_hashmap(
    proto_struct: Option<prost_types::Struct>,
) -> HashMap<String, serde_json::Value> {
    proto_struct
        .map(|s| {
            s.fields
                .into_iter()
                .filter_map(|(k, v)| proto_value_to_json_value(v).map(|json_val| (k, json_val)))
                .collect()
        })
        .unwrap_or_default()
}

/// Convert HashMap<String, serde_json::Value> to proto Struct
pub fn hashmap_to_proto_struct(map: &HashMap<String, serde_json::Value>) -> prost_types::Struct {
    let fields = map
        .iter()
        .filter_map(|(k, v)| json_value_to_proto_value(v).map(|proto_val| (k.clone(), proto_val)))
        .collect();

    prost_types::Struct { fields }
}

/// Convert proto Value to serde_json::Value
fn proto_value_to_json_value(value: prost_types::Value) -> Option<serde_json::Value> {
    use prost_types::value::Kind;

    match value.kind? {
        Kind::NullValue(_) => Some(serde_json::Value::Null),
        Kind::NumberValue(n) => Some(serde_json::Value::Number(serde_json::Number::from_f64(n)?)),
        Kind::StringValue(s) => Some(serde_json::Value::String(s)),
        Kind::BoolValue(b) => Some(serde_json::Value::Bool(b)),
        Kind::StructValue(s) => {
            let map = proto_struct_to_hashmap(Some(s));
            Some(serde_json::Value::Object(map.into_iter().collect()))
        }
        Kind::ListValue(l) => {
            let values: Option<Vec<_>> = l
                .values
                .into_iter()
                .map(proto_value_to_json_value)
                .collect();
            Some(serde_json::Value::Array(values?))
        }
    }
}

/// Convert serde_json::Value to proto Value
fn json_value_to_proto_value(value: &serde_json::Value) -> Option<prost_types::Value> {
    use prost_types::value::Kind;

    let kind = match value {
        serde_json::Value::Null => Kind::NullValue(0),
        serde_json::Value::Bool(b) => Kind::BoolValue(*b),
        serde_json::Value::Number(n) => Kind::NumberValue(n.as_f64()?),
        serde_json::Value::String(s) => Kind::StringValue(s.clone()),
        serde_json::Value::Array(arr) => {
            let values: Option<Vec<_>> = arr.iter().map(json_value_to_proto_value).collect();
            Kind::ListValue(prost_types::ListValue { values: values? })
        }
        serde_json::Value::Object(obj) => {
            let map: HashMap<String, serde_json::Value> =
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            Kind::StructValue(hashmap_to_proto_struct(&map))
        }
    };

    Some(prost_types::Value { kind: Some(kind) })
}

impl From<ImageSource> for ProtoImageSource {
    fn from(source: ImageSource) -> Self {
        match source {
            ImageSource::Url(url) => ProtoImageSource {
                proto_source_type: Some(ProtoSourceType::Url(url)),
            },
            ImageSource::DynamicImage(image) => ProtoImageSource {
                proto_source_type: Some(ProtoSourceType::Data(image.into_bytes())),
            },
        }
    }
}
