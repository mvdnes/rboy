use serde::{Deserialize, Serialize};

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum GbMode {
    Classic,
    Color,
    ColorAsClassic,
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum GbSpeed {
    Single = 1,
    Double = 2,
}
