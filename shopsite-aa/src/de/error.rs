use std::{
	borrow::Cow,
	io,
	num::{ParseFloatError, ParseIntError},
	rc::Rc,
	path::Path,
	str::ParseBoolError
};
use super::Position;

/// Takes an `Option<Rc<Path>>` (like in the `Position` type) and turns it into a `str`.
pub(super) fn rc_path_to_str(file: &Option<Rc<Path>>) -> Cow<str> {
	if let Some(ref file) = file {
		file.as_os_str().to_string_lossy()
	}
	else {
		Cow::Borrowed("<unknown>")
	}
}

/// An error that occurred during reading, parsing, or deserialization.
#[derive(Debug, derive_more::Display, derive_more::Error)]
#[non_exhaustive]
pub enum Error {
	Other(#[error(ignore)] Cow<'static, str>),

	#[display(fmt = "{}: I/O error: {}", "rc_path_to_str(file)", error)]
	Io {
		error: io::Error,
		file: Option<Rc<Path>>
	},

	#[display(fmt = "{}: {}", pos, error)]
	InvalidBool {
		error: ParseBoolError,
		pos: Position
	},

	#[display(fmt = "{}: {}", pos, error)]
	InvalidFloat {
		error: ParseFloatError,
		pos: Position
	},

	#[display(fmt = "{}: {}", pos, error)]
	InvalidInt {
		error: ParseIntError,
		pos: Position
	},

	#[display(fmt = "{}: unexpected text before end of file", pos)]
	UnexpectedText {
		pos: Position
	}
}

impl serde::de::Error for Error {
	fn custom<T: std::fmt::Display>(msg: T) -> Self {
		Error::Other(msg.to_string().into())
	}
}

pub type Result<T> = std::result::Result<T, Error>;
