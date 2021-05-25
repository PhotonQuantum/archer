use chrono::NaiveDateTime;
use serde::{self, Deserialize, Deserializer, Serializer};

pub(crate) fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(date.timestamp())
}

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp = i64::deserialize(deserializer)?;
    Ok(NaiveDateTime::from_timestamp(timestamp, 0))
}
