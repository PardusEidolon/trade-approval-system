uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[uniffi::export]
pub fn sled_db() {
    let db = sled::Config::new()
        .temporary(true)
        .create_new(false)
        .mode(sled::Mode::HighThroughput)
        .open()
        .unwrap();

    // instert a test value
    db.insert(b"0", "python rocks!").unwrap();

    if let Some(vec) = db.get(b"0").unwrap() {
        // reconstruct str must be utf8
        let str = String::from_utf8(vec.to_vec()).expect("string is not utf8");
        println!("sled says: {:?}", str);
        return ();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_sled() {
        sled_db();
    }
}
