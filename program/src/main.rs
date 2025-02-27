#![no_main]
sp1_zkvm::entrypoint!(main);

use std::str::from_utf8;
use wasmi::{core::Trap, AsContextMut, Caller, Engine, Func, Linker, Module, Store};

#[derive(Default)]
struct HostState {
    input: Vec<u8>,
    output: Vec<u8>,
}

pub fn main() {
    let wasm = sp1_zkvm::io::read_vec();
    let input = sp1_zkvm::io::read_vec();
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm).unwrap();
    let mut store = Store::new(
        &engine,
        HostState {
            input,
            ..Default::default()
        },
    );
    let input_size = Func::wrap(&mut store, |caller: Caller<'_, HostState>| -> u32 {
        caller.data().input.len() as u32
    });
    let exit = Func::wrap(
        &mut store,
        |caller: Caller<'_, HostState>, exit_code: i32| {
            if exit_code == -71 {
                println!(
                    "panic message: {}",
                    from_utf8(&caller.data().output).unwrap_or_default()
                );
            }
            panic!("exit code: {}", exit_code);
        },
    );
    let read_input = Func::wrap(
        &mut store,
        |caller: Caller<'_, HostState>, target: u32, offset: u32, length: u32| {
            let global_memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let input = caller.data().input[(offset as usize)..(offset as usize + length as usize)]
                .to_vec();
            global_memory
                .write(caller, target as usize, &input)
                .unwrap();
        },
    );
    let write_output = Func::wrap(
        &mut store,
        |mut caller: Caller<'_, HostState>, offset: u32, length: u32| {
            let global_memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut output = vec![0u8; length as usize];
            global_memory
                .read(caller.as_context_mut(), offset as usize, &mut output)
                .unwrap();
            println!("output: {:?}", from_utf8(&output).unwrap_or_default());
            caller.data_mut().output.extend_from_slice(&output);
        },
    );
    let mut linker = <Linker<HostState>>::new(&engine);
    linker
        .define("fluentbase_v1preview", "_exit", exit)
        .unwrap();
    linker
        .define("fluentbase_v1preview", "_input_size", input_size)
        .unwrap();
    linker
        .define("fluentbase_v1preview", "_read", read_input)
        .unwrap();
    linker
        .define("fluentbase_v1preview", "_write", write_output)
        .unwrap();
    let instance = linker
        .instantiate(&mut store, &module)
        .unwrap()
        .start(&mut store)
        .unwrap();
    let main = instance.get_typed_func::<(), ()>(&store, "main").unwrap();
    main.call(&mut store, ()).unwrap();
    sp1_zkvm::io::commit_slice(&store.data().output);
}
