pub mod schema_parser;
pub mod rust_generator;
pub mod idl;

use anyhow::Result;
pub use schema_parser::*;
pub use rust_generator::*;
pub use idl::*;

/// Generate Rust code from WIND IDL schema
pub fn generate_rust_types(idl: &str) -> Result<String> {
    let schema = parse_idl(idl)?;
    let generator = RustGenerator::new();
    generator.generate(&schema)
}
