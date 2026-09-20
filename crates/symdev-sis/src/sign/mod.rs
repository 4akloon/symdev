//! DSA-SHA1 signing and verification for a SIS controller (type 36/37/38/39 fields).
mod base64;
mod openssl_decrypt;
mod private_key;
mod sign_key;
mod verify;

#[cfg(test)]
mod tests;
