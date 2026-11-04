use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let kernel: PathBuf = env::vars_os()
        .find_map(|(key, value)| {
            if key.to_string_lossy().starts_with("CARGO_BIN_FILE_") {
                Some(value)
            } else {
                None
            }
        })
        .expect("kernel artifact path was not provided by Cargo")
        .into();
    let image = Path::new("target/micro-os-bios.img");
    bootloader::DiskImageBuilder::new(kernel.clone())
        .create_bios_image(image)
        .expect("failed to create BIOS disk image");

    let target = Path::new("target");
    let uefi_partition = target.join("micro-os-uefi.img");
    let iso_root = target.join("micro-os-iso-root");
    let iso = target.join("micro-os.iso");
    fs::create_dir_all(&iso_root).expect("failed to create ISO staging directory");
    bootloader::DiskImageBuilder::new(kernel)
        .create_uefi_fat_partition(&uefi_partition)
        .expect("failed to create UEFI boot partition");
    let _ = iso_root;
    create_uefi_iso(&uefi_partition, &iso).expect("failed to create UEFI El Torito ISO");
}

fn create_uefi_iso(uefi_partition: &Path, iso: &Path) -> std::io::Result<()> {
    const BLOCK_SIZE: usize = 2048;
    const BOOT_CATALOG_BLOCK: usize = 19;
    const ROOT_DIRECTORY_BLOCK: usize = 20;
    const PATH_TABLE_BLOCK: usize = 21;
    const BOOT_IMAGE_BLOCK: usize = 22;

    let boot_image = fs::read(uefi_partition)?;
    let boot_image_blocks = boot_image.len().div_ceil(BLOCK_SIZE);
    let image_blocks = BOOT_IMAGE_BLOCK + boot_image_blocks;
    let mut image = vec![0; image_blocks * BLOCK_SIZE];
    let boot_start = BOOT_IMAGE_BLOCK * BLOCK_SIZE;
    image[boot_start..boot_start + boot_image.len()].copy_from_slice(&boot_image);

    let primary = 16 * BLOCK_SIZE;
    image[primary] = 1;
    image[primary + 1..primary + 6].copy_from_slice(b"CD001");
    image[primary + 6] = 1;
    copy_padded(&mut image[primary + 8..primary + 40], b"MICRO-OS", b' ');
    copy_padded(
        &mut image[primary + 40..primary + 72],
        b"MICRO_OS_UEFI",
        b' ',
    );
    set_both_u32(&mut image, primary + 80, image_blocks as u32);
    set_both_u16(&mut image, primary + 120, 1);
    set_both_u16(&mut image, primary + 124, 1);
    set_both_u16(&mut image, primary + 128, BLOCK_SIZE as u16);
    set_both_u32(&mut image, primary + 132, 10);
    write_u32_le(&mut image, primary + 140, PATH_TABLE_BLOCK as u32);
    image[primary + 144..primary + 148].fill(0);
    image[primary + 148..primary + 152].copy_from_slice(&(PATH_TABLE_BLOCK as u32).to_be_bytes());
    image[primary + 152..primary + 156].fill(0);
    image[primary + 156] = 34;
    set_both_u32(&mut image, primary + 158, ROOT_DIRECTORY_BLOCK as u32);
    set_both_u32(&mut image, primary + 166, BLOCK_SIZE as u32);
    image[primary + 190] = 1;
    image[primary + 191] = 0;

    let root = ROOT_DIRECTORY_BLOCK * BLOCK_SIZE;
    directory_record(&mut image, root, ROOT_DIRECTORY_BLOCK as u32, 0);
    directory_record(&mut image, root + 34, ROOT_DIRECTORY_BLOCK as u32, 1);

    let path_table = PATH_TABLE_BLOCK * BLOCK_SIZE;
    image[path_table] = 1;
    image[path_table + 2..path_table + 6]
        .copy_from_slice(&(ROOT_DIRECTORY_BLOCK as u32).to_le_bytes());
    image[path_table + 6..path_table + 8].copy_from_slice(&1u16.to_le_bytes());

    let boot_record = 17 * BLOCK_SIZE;
    image[boot_record + 1..boot_record + 6].copy_from_slice(b"CD001");
    image[boot_record + 6] = 1;
    copy_padded(
        &mut image[boot_record + 7..boot_record + 39],
        b"EL TORITO SPECIFICATION",
        b' ',
    );
    write_u32_le(&mut image, boot_record + 71, BOOT_CATALOG_BLOCK as u32);

    image[18 * BLOCK_SIZE + 1..18 * BLOCK_SIZE + 6].copy_from_slice(b"CD001");
    image[18 * BLOCK_SIZE] = 255;
    image[18 * BLOCK_SIZE + 6] = 1;

    let catalog = BOOT_CATALOG_BLOCK * BLOCK_SIZE;
    image[catalog] = 1;
    image[catalog + 1] = 0;
    copy_padded(
        &mut image[catalog + 4..catalog + 28],
        b"MICRO-OS UEFI BOOT",
        b' ',
    );
    image[catalog + 30] = 0x55;
    image[catalog + 31] = 0xaa;
    let checksum = (0..16)
        .filter(|index| *index != 14)
        .fold(0u16, |sum, index| {
            sum.wrapping_add(u16::from_le_bytes([
                image[catalog + index * 2],
                image[catalog + index * 2 + 1],
            ]))
        });
    let checksum = checksum.wrapping_neg();
    image[catalog + 28..catalog + 30].copy_from_slice(&checksum.to_le_bytes());

    let section = catalog + 32;
    image[section] = 0x91;
    image[section + 1] = 0xef;
    image[section + 2..section + 4].copy_from_slice(&1u16.to_le_bytes());
    copy_padded(
        &mut image[section + 4..section + 32],
        b"MICRO-OS UEFI",
        b' ',
    );

    let entry = catalog + 64;
    image[entry] = 0x88;
    image[entry + 6..entry + 8].copy_from_slice(&((boot_image.len() / 512) as u16).to_le_bytes());
    write_u32_le(&mut image, entry + 8, BOOT_IMAGE_BLOCK as u32);

    fs::write(iso, image)
}

fn copy_padded(destination: &mut [u8], source: &[u8], padding: u8) {
    destination.fill(padding);
    destination[..source.len()].copy_from_slice(source);
}

fn directory_record(image: &mut [u8], offset: usize, extent: u32, identifier: u8) {
    image[offset] = 34;
    set_both_u32(image, offset + 2, extent);
    set_both_u32(image, offset + 10, 2048);
    image[offset + 25] = 2;
    set_both_u16(image, offset + 28, 1);
    image[offset + 32] = 1;
    image[offset + 33] = identifier;
}

fn write_u32_le(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn set_both_u16(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    buffer[offset + 2..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn set_both_u32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    buffer[offset + 4..offset + 8].copy_from_slice(&value.to_be_bytes());
}
