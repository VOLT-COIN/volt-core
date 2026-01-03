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

pub struct Wallet {
    pub private_key: SigningKey,
    pub public_key: VerifyingKey,
    pub mnemonic: Option<String>,
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
        
        (Wallet { private_key, public_key, mnemonic: Some(phrase.clone()) }, phrase)
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
        
        Ok(Wallet { private_key, public_key, mnemonic: Some(phrase.to_string()) })
    }



    pub fn new() -> Self {
        // Try load default
        if let Ok(contents) = fs::read_to_string("wallet.key") {
            // Try JSON First
            if let Ok(data) = serde_json::from_str::<WalletData>(&contents) {
                 if let Ok(bytes) = hex::decode(&data.key) {
                     if let Ok(private_key) = SigningKey::from_bytes(bytes.as_slice().into()) {
                         let public_key = VerifyingKey::from(&private_key);
                         return Wallet { private_key, public_key, mnemonic: data.mnemonic };
                     }
                 }
            }
            // Fallback: Hex (Old Format)
             if let Ok(bytes) = hex::decode(contents.trim()) {
                 if let Ok(private_key) = SigningKey::from_bytes(bytes.as_slice().into()) {
                     let public_key = VerifyingKey::from(&private_key);
                     return Wallet { private_key, public_key, mnemonic: None };
                 }
             }
        }
        
        // Generate new with mnemonic by default
        let (wallet, _) = Wallet::create_with_mnemonic();
        wallet.save();
        wallet
    }

    pub fn save(&self) {
        let hex_key = hex::encode(self.private_key.to_bytes());
        let data = WalletData { 
            key: hex_key, 
            mnemonic: self.mnemonic.clone() 
        };
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            fs::write("wallet.key", json).expect("Failed to write wallet.key");
        }
    }

    pub fn save_encrypted(&self, password: &str) {
        let hex_key = hex::encode(self.private_key.to_bytes());
        let mut encrypted = encrypt_data(hex_key.as_bytes(), password);
        encrypted.mnemonic = self.mnemonic.clone(); // Store mnemonic in encrypted file

        let json = serde_json::to_string(&encrypted).unwrap();
        fs::write("wallet.enc", json).expect("Failed to write wallet.enc");
        // Optionally delete wallet.key
        let _ = fs::remove_file("wallet.key");
    }

    pub fn load_encrypted(password: &str) -> Option<Self> {
        if let Ok(contents) = fs::read_to_string("wallet.enc") {
            if let Ok(data) = serde_json::from_str::<EncryptedData>(&contents) {
                if let Ok(plaintext) = decrypt_data(&data, password) {
                     if let Ok(hex_str) = String::from_utf8(plaintext) {
                         if let Ok(bytes) = hex::decode(hex_str.trim()) {
                             if let Ok(private_key) = SigningKey::from_bytes(bytes.as_slice().into()) {
                                 let public_key = VerifyingKey::from(&private_key);
                                 return Some(Wallet { private_key, public_key, mnemonic: data.mnemonic });
                             }
                         }
                     }
                }
            }
        }
        None
    }

    pub fn get_address(&self) -> String {
        hex::encode(self.public_key.to_sec1_bytes())
    }

    #[allow(dead_code)]
    pub fn sign(&self, message: &str) -> String {
        use k256::ecdsa::signature::Signer;
        let signature: k256::ecdsa::Signature = self.private_key.sign(message.as_bytes());
        hex::encode(signature.to_bytes())
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
