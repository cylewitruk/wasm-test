use crate::{runtime::Stack, tests::runtime::helpers::*};
use clarity::vm::Value;

#[test]
fn test_new_ptr() {
    let stack = Stack::default();
    let mut store = get_new_store(stack);

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
