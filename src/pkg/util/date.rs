use chrono::{DateTime, Utc};
use overloadf::*;
use std::time::SystemTime;

#[overload]
pub fn date_format_to_second(date: SystemTime) -> String {
    let date = DateTime::<Utc>::from(date);
    let df = "%a, %-e %b %Y %H:%M:%S GMT";
    let tag = date.format(df).to_string();
    tag
}
#[overload]
pub fn date_format_to_second(date: DateTime<Utc>) -> String {
    let df = "%a, %-e %b %Y %H:%M:%S GMT";
    let tag = date.format(df).to_string();
    tag
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test1() {
        let datetime: DateTime<Utc> = Utc::now();

        // 格式化日期为 "Mon, 2 Jan 2006 15:04:05 GMT"
        let formatted_date = date_format_to_second(datetime);
        println!("{}", formatted_date);
    }
}
