#![allow(warnings)]

use std::string::String;

use sha256::digest;

fn main() -> anyhow::Result<()> {
    let db = sled::open("sled")?;

    db.clear();

    for i in (0..10) {
        db.insert(digest(&[i]), vec![i]);
    }

    for t in db.iter() {
        let item = &t?;
        // let v_b = &t?.1[..];
        let k = String::from_utf8_lossy(&item.0[..]);
        println!("{} -> {:?}", k, item.1);
    }
    Ok(())
}
