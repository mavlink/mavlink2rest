mod convert;

pub use convert::{legacy_to_modern, modern_to_legacy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonSchema {
    Modern,
    Legacy,
}
