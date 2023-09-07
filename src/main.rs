
use clarity::vm::Value;
use wasmtime::{AsContextMut, ExternRef, Config, Engine, Store, Module, Extern, Instance, Val, Func};

mod wasm_generator;

fn main() {
    let wasm_bytes = wasm_generator::generate_wasm();
    let state = MyApplicationState{};

    let mut config = Config::new();
    config.wasm_reference_types(true);

    let engine = Engine::new(&config).expect("Failed to initialize engine");
    let mut store = Store::new(&engine, state);
    let module = Module::from_binary(&store.engine(), &wasm_bytes)
        .expect("Failed to load module");

    let param_a = Some(ExternRef::new(Value::Int(1)));
    let param_b = Some(ExternRef::new(Value::Int(2)));

    let func = Func::wrap(store.as_context_mut(), |a: Option<ExternRef>, b: Option<ExternRef>| {
        let a = a.unwrap();
        let b = b.unwrap();
        let input_a = a.data().downcast_ref::<Value>().unwrap();
        let input_b = b.data().downcast_ref::<Value>().unwrap();
        println!("Input: a={:?}, b={:?}", input_a, input_b);

        let result = Value::Int(input_a.clone().expect_i128() + input_b.clone().expect_i128());
        println!("Inner result: {:?}", result);
        
        let retopt: Option<ExternRef> = Some(ExternRef::new(result));
        println!("Inner ret: {:?}", retopt);

        Ok(retopt)
    });
    let add = Extern::Func(func);
    
    let instance = Instance::new(&mut store, &module, &[add])
        .expect("Couldn't create new module instance");

    let instance_fn = instance.get_func(&mut store, "toplevel")
        .expect("Failed to get fn");

    let result_val = Some(ExternRef::new(Value::Int(0)));
    let results = &mut [Val::ExternRef(result_val)];

    instance_fn.call(&mut store, 
        &[Val::ExternRef(param_a), Val::ExternRef(param_b)], 
        results)
        .expect("Failed to call function");

    println!("Results: {:?}", results);
    let result_unwrapped = results[0].unwrap_externref().unwrap();
    let result = result_unwrapped.data().downcast_ref::<Value>().unwrap();
    println!("Result: {:?}", result);
}

#[derive(Debug, Copy, Clone)]
struct MyApplicationState {
}
