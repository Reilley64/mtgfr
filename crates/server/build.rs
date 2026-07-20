//! Compiles `proto/mtgfr/v1/mtgfr.proto` (wire-protocol-and-visibility spec) into server stubs plus an encoded file
//! descriptor set, so `crates/server/src/grpc` has both the generated types/traits and (for a
//! future reflection service) the wire's self-description.

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .file_descriptor_set_path(out_dir.join("mtgfr_descriptor.bin"))
        .compile_protos(&["../../proto/mtgfr/v1/mtgfr.proto"], &["../../proto"])?;
    Ok(())
}
