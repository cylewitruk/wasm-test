use clarity::vm::Value;
use wasmtime::{Func, Caller};
use crate::tests::runtime::helpers::*;
use crate::runtime::{stack::StackFrame, ClarityWasmContext};


/// Test function
#[test]
fn test() {
    let mut store = get_new_store();

    let func = Func::wrap(&mut store, move |mut caller: Caller<'_, ClarityWasmContext>| {
        let stack = &caller.data().stack;
        stack.exec(&mut Vec::new(),
        |frame: StackFrame| {
            let ptr1 = frame.push(Value::Int(1));
            let ptr2 = frame.push(Value::UInt(2));

            let val1 = frame.get(ptr1);
            let val2 = frame.get(ptr2);

            println!("ptr1={:?}, val1={:?}", ptr1, val1);
            println!("ptr2={:?}, val2={:?}", ptr2, val2);

            vec![]
        });
    });

    func.call(&mut store, &[], &mut [])
        .expect("Failed to call function");


}
