//! Utility functions for hashing and serialization

use bech32::Bech32m;
use uuid7::uuid7;

// construct a unique user id then encode using bech32
pub fn new_uuid_to_bech32(hrp: &str) -> anyhow::Result<String> {
    let hrp = bech32::Hrp::parse(hrp)?;
    let encode = bech32::encode::<Bech32m>(hrp, uuid7().as_bytes())?;
    Ok(encode)
}
