fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=proto/piva.proto");
    
    prost_build::Config::new()
        .compile_protos(&["proto/piva.proto"], &["proto/"])?;
    Ok(())
}
