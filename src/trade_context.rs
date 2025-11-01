use super::witness_set::Witness;

#[derive(Debug)]
pub struct Tade {
    pub id: String,                // uuid7
    pub witness_set: Vec<Witness>, // trade context
}
