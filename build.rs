use std::env;
use std::path::Path;

fn main() {
    let kernel = env::vars_os()
        .find_map(|(key, value)| {
            if key.to_string_lossy().starts_with("CARGO_BIN_FILE_") {
                Some(value)
            } else {
                None
            }
        })
        .expect("kernel artifact path was not provided by Cargo");
    let image = Path::new("target/micro-os-bios.img");
    bootloader::DiskImageBuilder::new(kernel.into())
        .create_bios_image(image)
        .expect("failed to create BIOS disk image");
}
