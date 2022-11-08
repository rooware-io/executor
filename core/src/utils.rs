use rand::rngs::OsRng;
use solana_sdk::signature::Keypair;

/// Clone the given keypair.
pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}

/// Generate a random keypair.
pub fn random_keypair() -> Keypair {
    Keypair::generate(&mut OsRng::default())
}
