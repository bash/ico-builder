use ico_builder::IcoBuilder;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    IcoBuilder::default()
        .add_source_file("examples/icons/icon-32x32.png")
        .add_source_file("examples/icons/icon-256x256.png")
        .build_file("examples/icons/icon.ico")?;
    Ok(())
}
