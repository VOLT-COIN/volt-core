use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{TransactTo, U256, Address, ExecutionResult, Output, CreateScheme},
    EVM,
};
use std::str::FromStr;

pub struct EvmRunner {
    pub db: CacheDB<EmptyDB>,
}

impl EvmRunner {
    pub fn new() -> Self {
        EvmRunner {
            db: CacheDB::new(EmptyDB::default()),
        }
    }

    /// Execute Solidity Bytecode (Deploy or Call)
    /// `caller`: Sender Address (EVM formatted, e.g., 20 bytes)
    /// `to`: Target Contract (None for Deploy)
    /// `input`: Bytecode + Args
    /// `value`: Amount of Wei sent
    pub fn execute(
        &mut self,
        caller: String,
        to: Option<String>,
        input: Vec<u8>,
        value: u64,
    ) -> Result<(Option<String>, Vec<u8>, u64), String> {
        let mut evm = EVM::new();
        evm.database(self.db.clone());

        // 1. Convert Inputs
        // Volt Addresses are strings. For EVM we need 20-byte Address.
        // HACK: Hash string and take first 20 bytes? Or parse if hex?
        // If caller is "User1", we map it deterministically.
        let caller_addr = Self::volt_to_evm(&caller);
        let to_addr = to.map(|s| Self::volt_to_evm(&s));
        
        let val = U256::from(value);

        // 2. Configure Tx
        evm.env.tx.caller = caller_addr;
        evm.env.tx.transact_to = if let Some(addr) = to_addr {
            TransactTo::Call(addr)
        } else {
            TransactTo::Create(CreateScheme::Create) // Create new
        };
        evm.env.tx.data = input.into();
        evm.env.tx.value = val;
        evm.env.tx.gas_limit = 1_000_000; // Hardcoded Gas Limit
        evm.env.tx.gas_price = U256::from(1);

        // 3. Execute
        let result = evm.transact_commit().map_err(|e| format!("EVM Error: {:?}", e))?;

        // 4. Handle Result
        match result {
            ExecutionResult::Success { reason: _, gas_used, gas_refunded: _, logs: _, output } => {
                match output {
                    Output::Create(_bytes, address) => {
                         // Contract Deployed
                         // Return: (AddressString, Empty, GasUsed)
                         let addr_str = format!("{:?}", address.unwrap_or_default());
                         Ok((Some(addr_str), vec![], gas_used))
                    },
                    Output::Call(bytes) => {
                         Ok((None, bytes.to_vec(), gas_used))
                    }
                }
            },
            ExecutionResult::Revert { gas_used, output } => {
                Err(format!("Reverted (Gas: {}): {:?}", gas_used, output))
            },
            ExecutionResult::Halt { reason, gas_used } => {
                Err(format!("Halted (Gas: {}): {:?}", gas_used, reason))
            },
        }
    }
    
    // Helper: Map arbitrary string to H160
    fn volt_to_evm(volt_addr: &str) -> Address {
        // If already hex 0x..., parse it
        if volt_addr.starts_with("0x") && volt_addr.len() == 42 {
             if let Ok(addr) = Address::from_str(volt_addr) {
                 return addr;
             }
        }
        
        // Otherwise hash it
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(volt_addr.as_bytes());
        let result = hasher.finalize();
        // Take first 20 bytes
        Address::from_slice(&result[0..20])
    }
}
