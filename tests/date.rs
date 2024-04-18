use chrono::{DateTime, Utc};

#[cfg(test)]
mod test {
    use super::*;
    use chrono::NaiveDateTime;
    use log::info;
    use rs_s3_local::util::date::date_format_to_second;

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
