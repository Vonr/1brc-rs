use std::num::NonZeroI32;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct Data {
    pub total: i32,
    pub count: NonZeroI32,
    pub min: i16,
    pub max: i16,
}
