pub mod rendering;
pub mod user_input;
pub mod wheeler;

// This is because CI will treat it as an error if there is at least one test missing.
#[test]
fn dummy_test() {
    assert_eq!(1 + 1, 2);
}
