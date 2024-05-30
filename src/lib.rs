//! A crate for creating multi-size ICO files from separate images.
//! The images are automatically resized to the specified sizes.
//!
//! ## Examples
//! ### Basic
//! In this example, the 16px, 24px, and 32px versions of this icon will
//! be resized versions of `app-icon-32x32.png` while the 48px and 256px
//! versions will be resized from `app-icon-256x256.png`.
//!
//! ```no_run
//! # use ico_builder::IcoBuilder;
//! IcoBuilder::default()
//!     .add_source_file("app-icon-32x32.png")
//!     .add_source_file("app-icon-256x256.png")
//!     .build_file("app-icon.ico");
//! ```
//!
//! ### Custom Icon Sizes
//! If you want more fine grained control over which icon sizes are included,
//! you can specify a custom list of icon sizes.
//!
//! ```no_run
//! # use ico_builder::IcoBuilder;
//! IcoBuilder::default()
//!     .sizes(&[16, 32])
//!     .add_source_file("app-icon-32x32.png")
//!     .build_file("app-icon.ico");
//! ```

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::png::PngEncoder;
use image::imageops::resize;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ExtendedColorType, ImageEncoder};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::{env, iter};

mod error;
pub use error::*;
pub type Result<T> = std::result::Result<T, Error>;

pub use image::imageops::FilterType;

/// Builds an ICO file from individual files.
/// For each size, the closest source image is scaled down to the appropriate size.
#[derive(Debug)]
pub struct IcoBuilder {
    sizes: IconSizes,
    source_files: Vec<PathBuf>,
    filter_type: FilterType,
}

impl Default for IcoBuilder {
    fn default() -> Self {
        IcoBuilder {
            sizes: Default::default(),
            source_files: Default::default(),
            filter_type: FilterType::Lanczos3,
        }
    }
}

impl IcoBuilder {
    /// Customizes the sizes included in the ICO file. Defaults to [`IconSizes::MINIMAL`].
    pub fn sizes(&mut self, sizes: impl Into<IconSizes>) -> &mut IcoBuilder {
        self.sizes = sizes.into();
        self
    }

    /// Adds a source file. These file can be PNG, BMP or any other format supported by the
    /// [`image`] crate.
    /// The icons are assumed to be a square.
    ///
    /// Note that you'll have to enable the necessary features on the [`image`] crate if you want
    /// to use formats other than PNG or BMP:
    /// ```toml
    /// # ...
    ///
    /// [dependencies]
    /// ico-builder = { version = "...", features = ["jpeg"] }
    /// ```
    pub fn add_source_file(&mut self, source_file: impl AsRef<Path>) -> &mut IcoBuilder {
        self.add_source_files(iter::once(source_file))
    }

    /// Adds sources files. See: [`IcoBuilder::add_source_file`].
    pub fn add_source_files(
        &mut self,
        source_files: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> &mut IcoBuilder {
        self.source_files
            .extend(source_files.into_iter().map(|f| f.as_ref().to_owned()));
        self
    }

    /// Customizes the filter type used when downscaling the images. Defaults to [`FilterType::Lanczos3`].
    pub fn filter_type(&mut self, filter_type: FilterType) -> &mut IcoBuilder {
        self.filter_type = filter_type;
        self
    }

    /// Builds the ICO file and writes it to the specified `output_file_path`.
    pub fn build_file(&self, output_file_path: impl AsRef<Path>) -> Result<()> {
        let icons = decode_icons(&self.source_files)?;
        let frames = create_ico_frames(&self.sizes, &icons, self.filter_type)?;

        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&output_file_path)?;
        IcoEncoder::new(file).encode_images(&frames)?;

        Ok(())
    }

    /// Builds the ICO file and writes it to `OUT_DIR`.
    /// Tells Cargo to re-build when one of the specified sources changes.
    /// ## Panics
    /// This function panics if the path of one of the source files is not valid UTF-8.
    pub fn build_file_cargo(&self, file_name: impl AsRef<OsStr>) -> Result<PathBuf> {
        let out_dir = env::var_os("OUT_DIR").expect(
            "OUT_DIR environment variable is required.\nHint: This function is intended to be used in Cargo build scripts.",
        );
        let output_path: PathBuf = [&out_dir, file_name.as_ref()].iter().collect();

        for file in &self.source_files {
            println!(
                "cargo:rerun-if-changed={}",
                file.to_str().expect("Path needs to be valid UTF-8")
            )
        }

        self.build_file(&output_path)?;

        Ok(output_path)
    }
}

/// A list of icon sizes.
#[derive(Debug)]
pub struct IconSizes(Cow<'static, [u32]>);

impl IconSizes {
    /// The [bare minimum] recommended icon sizes: 16x16, 24x24, 32x32, 48x48, and 256x256.
    ///
    /// [bare minimum]: https://learn.microsoft.com/en-us/windows/apps/design/style/iconography/app-icon-construction#icon-scaling
    pub const MINIMAL: Self = Self::new(&[16, 24, 32, 48, 256]);

    pub const fn new(sizes: &'static [u32]) -> IconSizes {
        Self(Cow::Borrowed(sizes))
    }
}

impl Default for IconSizes {
    fn default() -> Self {
        IconSizes::MINIMAL
    }
}

impl<'a, I> From<I> for IconSizes
where
    I: IntoIterator<Item = &'a u32>,
{
    fn from(value: I) -> Self {
        IconSizes(value.into_iter().copied().collect::<Vec<_>>().into())
    }
}

impl Deref for IconSizes {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn decode_icons(
    icon_sources: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<Vec<DynamicImage>> {
    icon_sources
        .into_iter()
        .map(|path| decode_icon(path.as_ref()))
        .collect()
}

fn decode_icon(path: &Path) -> Result<DynamicImage> {
    let image = ImageReader::open(path)?.decode()?;

    if is_square(&image) {
        Ok(image)
    } else {
        Err(Error::NonSquareImage {
            path: path.to_owned(),
            width: image.width(),
            height: image.height(),
        })
    }
}

fn is_square(image: &DynamicImage) -> bool {
    image.width() == image.height()
}

fn find_next_bigger_icon(icons: &[DynamicImage], size: u32) -> Result<&DynamicImage> {
    icons
        .iter()
        .filter(|icon| icon.width() >= size)
        .min_by_key(|icon| icon.width())
        .ok_or(Error::MissingIconSize(size))
}

fn create_ico_frames(
    sizes: &IconSizes,
    icons: &[DynamicImage],
    filter_type: FilterType,
) -> Result<Vec<IcoFrame<'static>>> {
    sizes
        .iter()
        .copied()
        .map(|size| create_ico_frame(icons, size, filter_type))
        .collect()
}

fn create_ico_frame(
    icons: &[DynamicImage],
    size: u32,
    filter_type: FilterType,
) -> Result<IcoFrame<'static>> {
    let next_bigger_icon = find_next_bigger_icon(icons, size)?;
    let resized = resize(next_bigger_icon, size, size, filter_type);
    encode_ico_frame(resized.as_raw(), size)
}

fn encode_ico_frame(buffer: &[u8], size: u32) -> Result<IcoFrame<'static>> {
    let color_type = ExtendedColorType::Rgba8;
    let mut encoded = Vec::new();
    PngEncoder::new(Cursor::new(&mut encoded)).write_image(buffer, size, size, color_type)?;
    Ok(IcoFrame::with_encoded(encoded, size, size, color_type)?)
}
