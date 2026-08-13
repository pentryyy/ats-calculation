use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioData {
    pub mic1: Vec<u8>,
    pub mic2: Vec<u8>,
}
