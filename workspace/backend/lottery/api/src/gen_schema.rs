use std::io::{Result, Write};

fn main() -> Result<()> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/schema.graphql");
    let mut f = std::fs::File::create(&path)?;
    let sdl = api::sdl_export();
    f.write_all(&sdl.as_bytes())?;
    println!("Schema is written to {path}");
    Ok(())
}
