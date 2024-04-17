use chrono::{DateTime, Utc};


// 使用提供的日期时间和格式化字符串，生成格式化后的日期字符串。
pub fn date_format_to_second(date: DateTime<Utc>) -> String {
    let df = "%a, %-e %b %Y %H:%M:%S GMT";
    let tag = date.format(df).to_string();
    tag
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::NaiveDateTime;
    use log::info;

    #[test]
    fn test1() {
        let datetime: DateTime<Utc> = Utc::now();

        // 格式化日期为 "Mon, 2 Jan 2006 15:04:05 GMT"
        let formatted_date = date_format_to_second(datetime);
        info!("{}", formatted_date);
    }

    #[test]
    fn test2() {
        let date = "20240406T070323Z";
        let fmt = "%Y%m%dT%H%M%SZ";
        let x = NaiveDateTime::parse_from_str(date, fmt).unwrap();
        let x = x.and_utc();
        info!("{}", x);
    }
}
