use chrono::{DateTime, Utc};
use std::time::SystemTime;

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
