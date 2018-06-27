use super::{clap, toml, askama, html5ever_ext, image, simplelog};
use std::fmt::{self};
use std::io;

#[derive(Debug)]
pub struct OpaqueError {
    msg: String,
}

impl OpaqueError {
    pub fn new<S: Into<String>>(msg: S) -> OpaqueError {
        OpaqueError { msg: msg.into() }
    }
}

impl fmt::Display for OpaqueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

macro_rules! opaque_error {
    ($error:ty) => {
        impl From<$error> for OpaqueError {
            fn from(error: $error) -> Self {
                OpaqueError::new(format!("{}", error))
            }
        }
    }
}

opaque_error!(io::Error);
opaque_error!(clap::Error);
opaque_error!(toml::de::Error);
opaque_error!(toml::ser::Error);
opaque_error!(askama::Error);
opaque_error!(html5ever_ext::HtmlError);
opaque_error!(image::ImageError);
opaque_error!(simplelog::TermLogError);

pub type OResult<T> = Result<T, OpaqueError>;
