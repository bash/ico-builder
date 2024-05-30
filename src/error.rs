use core::fmt;
use std::path::PathBuf;
use std::{error, io};

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Image(image::ImageError),
    Io(io::Error),
    MissingIconSize(u32),
    NonSquareImage {
        path: PathBuf,
        width: u32,
        height: u32,
    },
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Image(e) => e.source(),
            Error::Io(e) => e.source(),
            Error::MissingIconSize(..) => None,
            Error::NonSquareImage { .. } => None,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Image(e) => e.fmt(f),
            Error::Io(e) => e.fmt(f),
            Error::MissingIconSize(size) => write!(f, "No icon in the sources is >= {size}px"),
            Error::NonSquareImage {
                path,
                width,
                height,
            } => write!(
                f,
                "Image {p} ({width} × {height}) is not a square",
                p = path.display()
            ),
        }
    }
}

impl From<image::ImageError> for Error {
    fn from(source: image::ImageError) -> Self {
        Error::Image(source)
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Error::Io(source)
    }
}
