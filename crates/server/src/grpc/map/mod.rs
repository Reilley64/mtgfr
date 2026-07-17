//! Boundary mappers between `schema`'s wire DTOs and the generated native-protobuf types in
//! [`crate::grpc::pb`] (ADR 0032). Each submodule mirrors one `.proto` file.

mod catalog;
mod common;
mod intent;
mod stream;

pub use catalog::*;
#[allow(unused_imports)] // re-exported for callers outside `grpc::map`; none exist yet.
pub use common::*;
pub use intent::*;
pub use stream::*;
