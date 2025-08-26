pub mod idl;
pub mod rust_generator;
pub mod schema_parser;

use anyhow::Result;
pub use idl::*;
pub use rust_generator::*;
pub use schema_parser::*;

/// Generate Rust code from WIND IDL schema
pub fn generate_rust_types(idl: &str) -> Result<String> {
    let schema = parse_idl(idl)?;
    let generator = RustGenerator::new();
    generator.generate(&schema)
}
