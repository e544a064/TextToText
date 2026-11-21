//! Build-time setup: embed the app icon into the Windows executable.
//!
//! Windows needs an `.ico` for the icon shown by Explorer, shortcuts, and the
//! taskbar. Instead of checking one in, generate it from `assets/icon.png` so
//! there is only a single source image to maintain. The window icon itself is
//! set at runtime by eframe on all platforms.

use std::path::{Path, PathBuf};

const ICON_PNG: &str = "assets/icon.png";

/// Sizes Windows picks from, depending on the display scaling and context.
const ICON_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

fn main() {
	println!("cargo:rerun-if-changed={ICON_PNG}");

	if std::env::var_os("CARGO_CFG_WINDOWS").is_none() {
		return;
	}

	let icon = generate_ico().expect("failed to generate a Windows icon from assets/icon.png");

	winresource::WindowsResource::new()
	. set_icon(icon.to_str().expect("icon path is not valid UTF-8"))
	. compile()
	. expect("Failed to compile Windows resources");
}

/// Converts the source PNG into a multi-resolution ICO in `OUT_DIR`.
fn generate_ico() -> Result<PathBuf, Box<dyn std::error::Error>> {
	use image::codecs::ico::{IcoEncoder, IcoFrame};
	use image::imageops::FilterType;
	use image::ExtendedColorType;

	let source = (
		image::open(manifest_path(ICON_PNG))?
		. to_rgba8()
	);
	let mut frames = Vec::with_capacity(ICON_SIZES.len());

	for size in ICON_SIZES {
		let image = image::imageops::resize(&source, size, size, FilterType::Lanczos3);
		frames.push(
			IcoFrame::as_png(image.as_raw(), size, size, ExtendedColorType::Rgba8)?
		);
	}

	let path = (
		PathBuf::from(std::env::var("OUT_DIR")?)
		. join("icon.ico")
	);
	let writer = std::io::BufWriter::new(std::fs::File::create(&path)?);
	IcoEncoder::new(writer).encode_images(&frames)?;

	Ok(path)
}

fn manifest_path(relative: &str) -> PathBuf {
	Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set"))
	. join(relative)
}
