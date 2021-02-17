use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "./src/bin"]
pub struct Shaders;
