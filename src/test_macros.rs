#[macro_export]
macro_rules! mapvec {
    (
        $i:path: $($x:expr),*
    ) => (
        vec!($($i($x)),*)
    )
}


#[test]
fn stringvec_test()
{
    assert_eq!(mapvec!(String::from: "yolo", "swag"), vec!(String::from("yolo"), String::from("swag")));
}
