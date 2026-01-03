use std::fs;
use k256::ecdsa::{SigningKey, VerifyingKey};

fn main() {
    // Logic copied from wallet.rs to derive public address
    if let Ok(contents) = fs::read_to_string("wallet.key") {
        let contents = contents.trim();
        // Try hex decode (Plaintext)
        if let Ok(bytes) = hex::decode(contents) {
             if let Ok(private_key) = SigningKey::from_bytes(bytes.as_slice().into()) {
                 let public_key = VerifyingKey::from(&private_key);
                 let address = hex::encode(public_key.to_sec1_bytes());
                 println!("FOUND_ADDRESS:{}", address);
                 return;
             }
        }
    }
    println!("ERROR: Could not read or parse wallet.key");
}
