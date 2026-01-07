//! # Archimedes Docs
//!
//! Automatic API documentation generation for the Archimedes framework.
//!
//! This crate provides:
//! - **OpenAPI spec generation** from Themis contracts
//! - **Swagger UI** endpoint for interactive API exploration
//! - **ReDoc** endpoint for beautiful API documentation
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use archimedes_docs::{OpenApiGenerator, SwaggerUi, ReDoc};
//! use archimedes_sentinel::Artifact;
//!
//! // Load your Themis contract
//! let artifact = Artifact::from_file("api.yaml")?;
//!
//! // Generate OpenAPI spec
//! let generator = OpenApiGenerator::new()
//!     .title("My API")
//!     .version("1.0.0")
//!     .description("My awesome API");
//! let spec = generator.generate(&artifact)?;
//!
//! // Serve documentation endpoints
//! let swagger = SwaggerUi::new("/docs", &spec);
//! let redoc = ReDoc::new("/redoc", &spec);
//! ```
//!
//! ## Features
//!
//! - **Contract-driven**: Generates documentation directly from Themis contracts
//! - **Schema validation**: Ensures API documentation matches implementation
//! - **Interactive**: Swagger UI allows testing API endpoints
//! - **Beautiful**: ReDoc provides clean, readable documentation
//! - **Customizable**: Configure titles, descriptions, servers, and more

mod error;
mod openapi;
mod redoc;
mod swagger;

pub use error::{DocsError, DocsResult};
pub use openapi::{
    Contact, Info, License, MediaType, OpenApi, OpenApiGenerator, Operation, Parameter,
    ParameterIn, PathItem, RequestBody, Response, Schema, SchemaType, Server, Tag,
};
pub use redoc::ReDoc;
pub use swagger::SwaggerUi;
