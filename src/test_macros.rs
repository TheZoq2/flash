use chrono::NaiveDateTime;

#[macro_export]
macro_rules! mapvec {
    (
        $i:path: $($x:expr),*
    ) => (
        vec!($($i($x)),*)
    )
}


pub fn naive_datetime_from_date(date_string: &str) -> ::chrono::ParseResult<NaiveDateTime> {
    NaiveDateTime::parse_from_str(&format!("{} 12:00:00", date_string), "%Y-%m-%d %H:%M:%S")
}


#[test]
fn stringvec_test() {
    assert_eq!(
        mapvec!(String::from: "yolo", "swag"),
        vec![String::from("yolo"), String::from("swag")]
    );
}
