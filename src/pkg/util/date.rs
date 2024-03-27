use chrono::{DateTime, Utc};
use std::time::SystemTime;
use serde::{Deserialize, Serializer};

pub fn date_format_to_second(date: SystemTime) -> String {
    let date = DateTime::<Utc>::from(date);
    let df = "%Y-%m-%d %H:%M:%S";
    let tag = date.format(df).to_string();
    tag
}

pub fn date_tag_to_second() -> String {
    let df = "%Y%m%d%H%M%S";
    let tag = Utc::now().format(df).to_string();
    tag
}

pub fn utc_date_format() -> String {
    let df = "%Y-%m-%dT%H:%M:%S.000Z";
    let tag = Utc::now().format(df).to_string();
    tag
}
pub fn serialize_date<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    let formatted_date = date.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&formatted_date)
}
pub fn deserialize_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: serde::Deserializer<'de>,
{
    let date_str = String::deserialize(deserializer)?;
    DateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
        .map_err(serde::de::Error::custom)
        .map(|datetime| datetime.with_timezone(&Utc))
}
