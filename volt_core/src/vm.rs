use wasmer::{Store, Module, Instance, Value, Function, FunctionEnv, imports};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Contract State: Key-Value Store
pub type Storage = HashMap<String, Vec<u8>>;

#[derive(Clone)]
pub struct ContractEnv {
    pub _storage: Arc<Mutex<Storage>>,
}

pub struct WasmVM {
    store: Store,
    instance: Instance,
    _env: FunctionEnv<ContractEnv>,
}

impl WasmVM {
    pub fn new(wasm_code: &[u8], storage: Storage) -> Result<Self, String> {
        let mut store = Store::default();
        let module = Module::new(&store, wasm_code).map_err(|e| e.to_string())?;

        let env_data = ContractEnv {
            storage: Arc::new(Mutex::new(storage)),
        };
        let env = FunctionEnv::new(&mut store, env_data);

        // Define Host Functions
        // 1. storage_read(key_ptr, key_len) -> val_ptr
        // 2. storage_write(key_ptr, key_len, val_ptr, val_len)
        
        // For MVP, straightforward imports (empty for now or basic print)
        let import_object = imports! {
            "env" => {
                "print" => Function::new_typed(&mut store, |val: i32| {
                    println!("Contract Print: {}", val);
                }),
            },
        };

        let instance = Instance::new(&mut store, &module, &import_object).map_err(|e| e.to_string())?;

        Ok(Self {
            store,
            instance,
            _env: env,
        })
    }

    pub fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Box<[Value]>, String> {
        let func = self.instance.exports.get_function(method).map_err(|e| e.to_string())?;
        func.call(&mut self.store, &args).map_err(|e| e.to_string())
    }

    // Retrieve storage after execution
    pub fn _get_storage(&self, _store: &mut Store) -> Storage {
        // In a real implementation, we would access the Env
        // But FunctionEnv access requires the store reference that owns it
        // MVP: Just returning empty or need to implement host function correctly to mutate Env
        HashMap::new() // Placeholder
    }
}
