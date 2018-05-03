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

#[cfg(test)]
macro_rules! db_test {
    ($fn_name:ident($fdbname:ident) $function:tt) => {
        #[test]
        fn $fn_name() {
            mod internal {
                use super::*;
                pub fn testfun($fdbname: &mut FileDatabase)
                    $function
            }
            ::file_database::db_test_helpers::run_test(internal::testfun);
        }
    }
}


#[test]
fn stringvec_test() {
    assert_eq!(
        mapvec!(String::from: "yolo", "swag"),
        vec![String::from("yolo"), String::from("swag")]
    );
}
