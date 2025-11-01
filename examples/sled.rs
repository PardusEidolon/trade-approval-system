#![allow(warnings)]

// Draft -> Either Approve Cancel -> Either Execute Cancel -> Trade

fn main() -> anyhow::Result<()> {
    let db = sled::open("./db/sled")?;

    Ok(())
}
