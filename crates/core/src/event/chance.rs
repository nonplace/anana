use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ChanceTemplate {
    Accident,
    Discovery,
    Conflict,
    Windfall,
}
