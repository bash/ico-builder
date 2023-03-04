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
//!     .source_files(&["app-icon-32x32.png", "app-icon-256x256.png"])
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
//!     .source_files(&["app-icon-32x32.png"])
//!     .build_file("app-icon.ico");
//! ```

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::png::PngEncoder;
use image::imageops::{resize, FilterType};
use image::io::Reader as ImageReader;
use image::{ColorType, DynamicImage, ImageEncoder};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::{env, io};
use thiserror::Error;

/// Builds an ICO file from individual files.
/// For each size, the closest source image is scaled down to the appropriate size.
#[derive(Debug, Default)]
pub struct IcoBuilder {
    sizes: IconSizes,
    source_files: Vec<PathBuf>,
}

impl IcoBuilder {
    /// Customizes the sizes included in the ICO file. Defaults to [`IconSizes::MINIMAL`].
    pub fn sizes(&mut self, sizes: impl Into<IconSizes>) -> &mut IcoBuilder {
        self.sizes = sizes.into();
        self
    }

    /// Adds sources files. These files can be PNG, BMP or any other format supported by the
    /// [`image`] crate.
    /// The source icons are assumed to be squares.
    ///
    /// Note that you'll have to enable the necessary features on the [`image`] crate if you want
    /// to use formats other than PNG or BMP:
    /// ```toml
    /// # ...
    ///
    /// [dependencies]
    /// ico-builder = { version = "...", features = ["image/jpeg"] }
    /// ```
    pub fn source_files(
        &mut self,
        source_files: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> &mut IcoBuilder {
        self.source_files
            .extend(source_files.into_iter().map(|f| f.as_ref().to_owned()));
        self
    }

    /// Adds a source file. See: [`IcoBuilder::source_files`].
    pub fn source_file(&mut self, source_file: impl AsRef<Path>) -> &mut IcoBuilder {
        self.source_files.push(source_file.as_ref().to_owned());
        self
    }

    /// Builds the ICO file and writes it to the specified `output_file_path`.
    pub fn build_file(&self, output_file_path: impl AsRef<Path>) -> Result<()> {
        let icons = decode_icons(&self.source_files)?;

        let frames: Vec<_> = self
            .sizes
            .0
            .iter()
            .copied()
            .map(|size| create_ico_frame(&icons, size))
            .collect::<std::result::Result<_, _>>()?;

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
    pub fn build_file_cargo(&self, file_name: impl AsRef<OsStr>) -> Result<PathBuf> {
        let out_dir = env::var("OUT_DIR").unwrap();
        let mut output_path = PathBuf::from(out_dir);
        output_path.push(file_name.as_ref());

        self.build_file(&output_path)?;

        Ok(output_path)
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("No icon in the sources is >= {0}px")]
    MissingIconSize(u32),
}

pub type Result<T> = std::result::Result<T, Error>;

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

fn decode_icons(
    icon_sources: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<Vec<DynamicImage>> {
    icon_sources
        .into_iter()
        .map(|path| ImageReader::open(path).unwrap().decode())
        .collect::<std::result::Result<_, _>>()
        .map_err(Into::into)
}

fn find_next_bigger_icon(icons: &[DynamicImage], size: u32) -> Result<&DynamicImage> {
    icons
        .iter()
        .filter(|icon| icon.width() >= size)
        .min_by_key(|icon| icon.width())
        .ok_or(Error::MissingIconSize(size))
}

fn create_ico_frame(icons: &[DynamicImage], size: u32) -> Result<IcoFrame<'static>> {
    let next_bigger_icon = find_next_bigger_icon(icons, size)?;
    let resized = resize(next_bigger_icon, size, size, FilterType::Lanczos3);
    encode_ico_frame(resized.as_raw(), size)
}

fn encode_ico_frame(buf: &[u8], size: u32) -> Result<IcoFrame<'static>> {
    let color_type = ColorType::Rgba8;
    let mut encoded = Vec::new();
    PngEncoder::new(Cursor::new(&mut encoded)).write_image(buf, size, size, color_type)?;
    Ok(IcoFrame::with_encoded(encoded, size, size, color_type)?)
}
