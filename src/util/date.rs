use chrono::{DateTime, Utc};

// 使用提供的日期时间和格式化字符串，生成格式化后的日期字符串。
pub fn date_format_to_second(date: DateTime<Utc>) -> String {
    let df = "%a, %-e %b %Y %H:%M:%S GMT";
    let tag = date.format(df).to_string();
    tag
}

