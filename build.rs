fn main() -> Result<(), Box<dyn std::error::Error>> {
    let capnp_dir = "capnproto/";

    let mut command = capnpc::CompilerCommand::new();

    command
        .src_prefix(capnp_dir)
        .import_path(capnp_dir);

    for entry in std::fs::read_dir(capnp_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("capnp") {
            let file_path = path.to_str()
                .ok_or_else(|| format!("Invalid UTF-8 in file path: {:?}", path))?;
            
            // Add file to compiler command
            command.file(file_path);
        }

    }

    command.run()?;

    Ok(())
}