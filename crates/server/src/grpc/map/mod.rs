//! Boundary mappers between `schema` DTOs and [`crate::grpc::pb`]. Each submodule mirrors one `.proto`.

mod catalog;
mod common;
mod intent;
mod stream;

pub use catalog::*;
pub use intent::*;
pub use stream::*;
