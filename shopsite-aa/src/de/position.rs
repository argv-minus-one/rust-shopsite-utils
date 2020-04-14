use std::{
	fmt::{Display, Formatter, Result as FmtResult},
	rc::Rc,
	path::Path
};
use super::rc_path_to_str;

/// Position in an input file where an error occurred.
// This structure is actually also used by the parser to keep track of where it's looking, not just for error reporting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
	/// Path to the file containing the error.
	pub file: Option<Rc<Path>>,

	/// Line on which the error appears.
	pub line: u32,

	/// Column on which the error appears.
	pub column: u32
}

impl Display for Position {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		write!(f, "{}:{}:{}", rc_path_to_str(&self.file), self.line, self.column)
	}
}
