use k256::ecdsa::{SigningKey, VerifyingKey};

use rand_core::OsRng;
use serde::{Serialize, Deserialize};
use std::fs;
// use aes_gcm::Nonce; // implicit in Aes256Gcm usage if needed, or explicitly import if used.
// checking code: I use `aes_gcm::Nonce` fully qualified in one place, but imported `Nonce` in another.
// Let's clean up imports.
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm
};
use sha2::{Sha256, Digest};
use hmac::Hmac;
use bip39::Mnemonic;



#[derive(Clone)] // Added Clone for easier handling if needed, though SigningKey might not be Clone? k256 SigningKey is Clone.
pub struct Wallet {
    pub private_key: Option<SigningKey>, // None if Locked
    pub public_key: Option<VerifyingKey>,
    pub mnemonic: Option<String>,
    pub is_locked: bool,
}

#[derive(Serialize, Deserialize)]
struct EncryptedData {
    salt: Vec<u8>,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    mnemonic: Option<String>, // New field
}

#[derive(Serialize, Deserialize)]
struct WalletData {
    key: String,
    mnemonic: Option<String>,
}

impl Wallet {
    pub fn create_with_mnemonic() -> (Self, String) {
        use rand_core::RngCore;
        let mut entropy = [0u8; 16];
        OsRng.fill_bytes(&mut entropy);

        let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();
        let phrase = mnemonic.to_string();
        let seed = mnemonic.to_seed("");
        
        let mut hasher = Sha256::new();
        hasher.update(seed); 
        let private_bytes = hasher.finalize();
        
        let private_key = SigningKey::from_slice(&private_bytes).unwrap();
        let public_key = VerifyingKey::from(&private_key);
        
        (Wallet { private_key: Some(private_key), public_key: Some(public_key), mnemonic: Some(phrase.clone()), is_locked: false }, phrase)
    }

    pub fn from_phrase(phrase: &str) -> Result<Self, String> {
        let mnemonic = Mnemonic::parse(phrase)
            .map_err(|e| format!("Invalid phrase: {}", e))?;
        let seed = mnemonic.to_seed("");
        
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let private_bytes = hasher.finalize();
        
        let private_key = SigningKey::from_slice(&private_bytes)
            .map_err(|e| format!("Invalid key from seed: {}", e))?;
        let public_key = VerifyingKey::from(&private_key);
        
        Ok(Wallet { private_key: Some(private_key), public_key: Some(public_key), mnemonic: Some(phrase.to_string()), is_locked: false })
    }



    pub fn new() -> Self {
        // 1. Check for Encrypted Wallet
        if std::path::Path::new("wallet.enc").exists() {
             return Wallet { private_key: None, public_key: None, mnemonic: None, is_locked: true };
        }

        // 2. Check for Legacy Plaintext
        if let Ok(contents) = fs::read_to_string("wallet.key") {
            // Try JSON First
            if let Ok(data) = serde_json::from_str::<WalletData>(&contents) {
                 if let Ok(bytes) = hex::decode(&data.key) {
                     if let Ok(private_key) = SigningKey::from_bytes(bytes.as_slice().into()) {
                         let public_key = VerifyingKey::from(&private_key);
                         return Wallet { private_key: Some(private_key), public_key: Some(public_key), mnemonic: data.mnemonic, is_locked: false };
                     }
                 }
            }
        }
        
        // 3. Defaults to Locked/Empty if no file found (Requires Setup via RPC)
        Wallet { private_key: None, public_key: None, mnemonic: None, is_locked: true }
    }

    pub fn save_encrypted(&self, password: &str) -> bool {
        if self.is_locked || self.private_key.is_none() { return false; }
        
        let hex_key = hex::encode(self.private_key.as_ref().unwrap().to_bytes());
        let mut encrypted = encrypt_data(hex_key.as_bytes(), password);
        encrypted.mnemonic = self.mnemonic.clone();

        let json = serde_json::to_string(&encrypted).unwrap();
        if fs::write("wallet.enc", json).is_ok() {
             // Successfully saved encrypted. Safe to delete legacy.
             let _ = fs::remove_file("wallet.key");
             return true;
        }
        false
    }
    
    pub fn unlock(&mut self, password: &str) -> bool {
        if let Ok(contents) = fs::read_to_string("wallet.enc") {
            if let Ok(data) = serde_json::from_str::<EncryptedData>(&contents) {
                if let Ok(plaintext) = decrypt_data(&data, password) {
                     if let Ok(hex_str) = String::from_utf8(plaintext) {
                         if let Ok(bytes) = hex::decode(hex_str.trim()) {
                             if let Ok(private_key) = SigningKey::from_bytes(bytes.as_slice().into()) {
                                 let public_key = VerifyingKey::from(&private_key);
                                 self.private_key = Some(private_key);
                                 self.public_key = Some(public_key);
                                 self.mnemonic = data.mnemonic;
                                 self.is_locked = false;
                                 return true;
                             }
                         }
                     }
                }
            }
        }
        false
    }

    pub fn _create(password: &str) -> Self {
        let (mut w, _) = Wallet::create_with_mnemonic();
        // private_key is Some by default from create_with_mnemonic helper refactor?
        // Wait, I need to check create_with_mnemonic implementation below.
        // It returns struct. I need to update it too.
        w.is_locked = false; 
        w.save_encrypted(password);
        w
    }

    pub fn get_address(&self) -> String {
        if let Some(pk) = &self.public_key {
            hex::encode(pk.to_sec1_bytes())
        } else {
            "LOCKED".to_string()
        }
    }

    pub fn _sign(&self, message: &str) -> Result<String, String> {
        if let Some(pk) = &self.private_key {
            use k256::ecdsa::signature::Signer;
            let signature: k256::ecdsa::Signature = pk.sign(message.as_bytes());
            Ok(hex::encode(signature.to_bytes()))
        } else {
            Err("Wallet is Locked or Private Key Missing".to_string())
        }
    }

    pub fn save(&self) {
        // Placeholder for consistency with API calls
        // In a real app, this might persist to disk if not using encrypted storage
    }
}

// Helpers
fn encrypt_data(data: &[u8], password: &str) -> EncryptedData {
    use aes_gcm::aead::rand_core::RngCore;
    
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    
    // Key Derivation
    let mut key = [0u8; 32]; // AES-256
    pbkdf2::pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt, 100_000, &mut key);
    
    let cipher = Aes256Gcm::new(&key.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, data).expect("Encryption failed");

    EncryptedData {
        salt: salt.to_vec(),
        nonce: nonce_bytes.to_vec(),
        ciphertext,
        mnemonic: None, 
    }
}

fn decrypt_data(data: &EncryptedData, password: &str) -> Result<Vec<u8>, aes_gcm::Error> {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &data.salt, 100_000, &mut key);
    
    let cipher = Aes256Gcm::new(&key.into());
    let nonce = aes_gcm::Nonce::from_slice(&data.nonce);
    
    cipher.decrypt(nonce, data.ciphertext.as_slice())
}
