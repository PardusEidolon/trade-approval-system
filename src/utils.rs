use bech32::Bech32;
use uuid7::uuid7;

// construct a unique user id then encode using bech32
pub fn uudi_to_bech32(hrp: &str) -> anyhow::Result<String> {
    let data = uuid7();
    let hrp = bech32::Hrp::parse(hrp)?;
    let encoded = bech32::encode::<Bech32>(hrp, data.as_bytes())?;

    Ok(encoded)
}
