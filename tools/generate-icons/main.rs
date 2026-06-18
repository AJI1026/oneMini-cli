//! 从 assets/logo.png 生成 Windows .ico（与 oneMini-web/public/logo/logo.png 同源）。
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let logo = root.join("assets/logo.png");
    let out = root.join("assets/onemini.ico");

    let img = image::open(&logo)?;
    let sizes = [256_u32, 128, 64, 48, 32, 16];
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for size in sizes {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();
        let image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
        icon_dir.add_entry(ico::IconDirEntry::encode(&image)?);
    }

    let file = File::create(&out)?;
    icon_dir.write(BufWriter::new(file))?;
    println!("wrote {}", out.display());
    Ok(())
}
