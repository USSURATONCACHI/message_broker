fn main() -> Result<(), Box<dyn std::error::Error>> {

    capnpc::CompilerCommand::new()
        .src_prefix("..")
        .file("../schema.capnp")
        .run()?;

    Ok(())
}