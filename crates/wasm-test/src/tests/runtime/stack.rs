use crate::runtime::{AsStack, Stack};
use crate::runtime::{stack::StackFrame, ClarityWasmContext};
use crate::tests::runtime::helpers::*;
use clarity::vm::Value;
use wasmtime::{Caller, Func};

/// Test function
#[test]
fn test() {
    let stack = Stack::default();
    let mut store = get_new_store(stack);

    let func = Func::wrap(&mut store, move |caller: Caller<'_, ClarityWasmContext>| {
        /*let stack = caller.as_stack();
        
        stack.exec(|frame: StackFrame| {
            let ptr1 = frame.push(Value::Int(1));
            let ptr2 = frame.push(Value::UInt(2));

            let val1 = frame.get(ptr1);
            let val2 = frame.get(ptr2);

            println!("ptr1={:?}, val1={:?}", ptr1, val1);
            println!("ptr2={:?}, val2={:?}", ptr2, val2);

            vec![]
        });*/
    });

    func.call(&mut store, &[], &mut [])
        .expect("Failed to call function");
}
