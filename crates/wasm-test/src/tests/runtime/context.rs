use clarity::vm::Value;
use crate::tests::runtime::helpers::*;

#[test]
fn test_new_ptr() {
    let mut store = get_new_store();

    let values = &mut store.data_mut().values;

    assert_eq!(0, values.new_ptr());
    assert_eq!(1, values.new_ptr());
    assert_eq!(2, values.new_ptr());
    values.drop(2);
    assert_eq!(2, values.push(Value::Int(1)));
    assert_eq!(3, values.new_ptr());
    assert_eq!(4, values.push(Value::Int(2)));
    assert_eq!(5, values.new_ptr());
    values.drop(5);
    assert_eq!(5, values.new_ptr());
}