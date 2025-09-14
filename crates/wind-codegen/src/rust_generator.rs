use crate::idl::*;
use anyhow::Result;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates type-safe Rust code from WIND IDL
pub struct RustGenerator {
    // Configuration options could go here
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RustGenerator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn generate(&self, idl: &WindIdl) -> Result<String> {
        let mut tokens = TokenStream::new();

        // Generate types
        for (name, type_def) in &idl.types {
            let type_tokens = self.generate_type(name, type_def)?;
            tokens.extend(type_tokens);
        }

        // Generate service traits
        for (name, service_def) in &idl.services {
            let service_tokens = self.generate_service(name, service_def)?;
            tokens.extend(service_tokens);
        }

        // Add common imports
        let imports = quote! {
            use wind_core::{WindValue, Result, WindError};
            use serde::{Serialize, Deserialize};
            use std::collections::HashMap;
        };

        let combined = quote! {
            #imports

            #tokens
        };

        Ok(combined.to_string())
    }

    fn generate_type(&self, name: &str, type_def: &TypeDefinition) -> Result<TokenStream> {
        let type_name = format_ident!("{}", name);

        match type_def {
            TypeDefinition::Struct { fields } => {
                let mut field_tokens = Vec::new();

                for (field_name, field_def) in fields {
                    let field_ident = format_ident!("{}", field_name);
                    let field_type_tokens = self.type_to_rust(&field_def.field_type)?;

                    if field_def.optional {
                        field_tokens.push(quote! {
                            pub #field_ident: Option<#field_type_tokens>
                        });
                    } else {
                        field_tokens.push(quote! {
                            pub #field_ident: #field_type_tokens
                        });
                    }
                }

                Ok(quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
                    pub struct #type_name {
                        #(#field_tokens,)*
                    }

                    impl From<#type_name> for WindValue {
                        fn from(val: #type_name) -> Self {
                            let mut map = HashMap::new();
                            // TODO: Add field conversions
                            WindValue::Map(map)
                        }
                    }

                    impl TryFrom<WindValue> for #type_name {
                        type Error = WindError;

                        fn try_from(value: WindValue) -> Result<Self> {
                            match value {
                                WindValue::Map(map) => {
                                    // TODO: Add field extractions
                                    Err(WindError::TypeMismatch {
                                        expected: stringify!(#type_name).to_string(),
                                        actual: format!("{:?}", value),
                                    })
                                }
                                _ => Err(WindError::TypeMismatch {
                                    expected: stringify!(#type_name).to_string(),
                                    actual: format!("{:?}", value),
                                })
                            }
                        }
                    }
                })
            }

            TypeDefinition::Enum { variants } => {
                let variant_tokens: Vec<_> =
                    variants.iter().map(|v| format_ident!("{}", v)).collect();

                Ok(quote! {
                    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
                    pub enum #type_name {
                        #(#variant_tokens,)*
                    }

                    impl From<#type_name> for WindValue {
                        fn from(val: #type_name) -> Self {
                            WindValue::String(format!("{:?}", val))
                        }
                    }

                    impl TryFrom<WindValue> for #type_name {
                        type Error = WindError;

                        fn try_from(value: WindValue) -> Result<Self> {
                            // TODO: Implement enum conversion
                            Err(WindError::TypeMismatch {
                                expected: stringify!(#type_name).to_string(),
                                actual: format!("{:?}", value),
                            })
                        }
                    }
                })
            }

            _ => Ok(TokenStream::new()), // Skip primitive types for now
        }
    }

    fn generate_service(&self, name: &str, service_def: &ServiceDefinition) -> Result<TokenStream> {
        let trait_name = format_ident!("{}Trait", name);
        let client_name = format_ident!("{}Client", name);
        let server_name = format_ident!("{}Server", name);

        // Generate trait methods
        let mut trait_methods = Vec::new();
        for (method_name, method_def) in &service_def.methods {
            let method_ident = format_ident!("{}", method_name);
            let param_type = self.type_to_rust(&method_def.params)?;
            let return_type = self.type_to_rust(&method_def.returns)?;

            trait_methods.push(quote! {
                async fn #method_ident(&self, params: #param_type) -> Result<#return_type>;
            });
        }

        Ok(quote! {
            #[async_trait::async_trait]
            pub trait #trait_name: Send + Sync {
                #(#trait_methods)*
            }

            pub struct #client_name {
                // TODO: Add client implementation
            }

            impl #client_name {
                pub fn new(registry_address: String) -> Self {
                    Self {}
                }
            }

            pub struct #server_name<T: #trait_name> {
                handler: T,
                // TODO: Add server implementation
            }

            impl<T: #trait_name> #server_name<T> {
                pub fn new(handler: T) -> Self {
                    Self { handler }
                }
            }
        })
    }

    fn type_to_rust(&self, type_def: &TypeDefinition) -> Result<TokenStream> {
        match type_def {
            TypeDefinition::Primitive { primitive_type } => {
                let rust_type = match primitive_type {
                    PrimitiveType::Bool => quote! { bool },
                    PrimitiveType::I32 => quote! { i32 },
                    PrimitiveType::I64 => quote! { i64 },
                    PrimitiveType::F32 => quote! { f32 },
                    PrimitiveType::F64 => quote! { f64 },
                    PrimitiveType::String => quote! { String },
                    PrimitiveType::Bytes => quote! { Vec<u8> },
                };
                Ok(rust_type)
            }
            TypeDefinition::Array { element_type } => {
                let element_rust_type = self.type_to_rust(element_type)?;
                Ok(quote! { Vec<#element_rust_type> })
            }
            TypeDefinition::Optional { inner_type } => {
                let inner_rust_type = self.type_to_rust(inner_type)?;
                Ok(quote! { Option<#inner_rust_type> })
            }
            TypeDefinition::Struct { .. } => {
                // For struct references, we'd need the name from context
                Ok(quote! { WindValue }) // Fallback
            }
            TypeDefinition::Enum { .. } => {
                // For enum references, we'd need the name from context
                Ok(quote! { WindValue }) // Fallback
            }
        }
    }
}
