use bootloader::BiosBoot;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let kernel = env!("CARGO_BIN_FILE_KERNEL_kernel");

    let bios_path = out_dir.join("bios.img");
    BiosBoot::new(&kernel).create_disk_image(bios_path).unwrap();

    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}