use bootloader::{BiosBoot, BootConfig};
use std::path::Path;

fn main() {
    let kernel = Path::new("./target/x86_64-failos/release/kernel");
    let mut bios = BiosBoot::new(kernel);

    let mut config = BootConfig::default();

    config.frame_buffer_logging = false;
    config.serial_logging = false;
    config.frame_buffer.minimum_framebuffer_height = Some(480);
    config.frame_buffer.minimum_framebuffer_width = Some(640);

    bios.set_boot_config(&config);

    bios.create_disk_image(Path::new("boot.img"))
        .expect("Failed to create disk image");

    println!("Bootable image created: boot.img");
}
