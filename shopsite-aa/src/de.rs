//! Deserializer implementation for ShopSite `.aa` files.
//! 
//! # Parsing Is Not Strict
//! 
//! Because there is no public specification for the format of `.aa` files, and all format details are inferred from the `.aa` files that ShopSite itself generates, this parser is not strict about what it will accept as valid. In particular, this parser will:
//! 
//! * Skip over lines containing only whitespace characters
//! * Allow comments to begin after any number of whitespace characters
//! * Understand `:` delimiters that are not followed by a space character
//! 
//! ShopSite itself may or may not be so forgiving. This parser is not designed to be used as a validator.
//! 
//! In other words, just because this parser doesn't reject or misunderstand a `.aa` file doesn't mean ShopSite won't reject or misunderstand it!

use serde::de::Deserialize;
use std::{
	fs::File,
	io::{self, BufRead, BufReader},
	path::Path,
	rc::Rc
};

mod position;
pub use position::*;

mod error;
pub use error::*;

mod parser_io;
use parser_io::*;

mod deser_toplevel;

mod deser_value;
use deser_value::*;

pub struct Deserializer<R: BufRead> {
	/// Source of input bytes.
	reader: R,

	/// Buffer of bytes read from the input source for the current line.
	/// 
	/// Parsing occurs at the byte level, since this format is always Windows-1252 and it's faster and simpler to parse byte-by-byte without dealing with UTF-8's variable-width characters.
	buf_b: Vec<u8>,

	/// Buffer of decoded text from the input source.
	/// 
	/// Note that this doesn't contain the entire line decoded. Rather, individual chunks of text are taken from `buf_b`, decoded, and then slices of this string are passed to the deserialize routines. This string is then cleared on every new line.
	buf_s: String,

	/// Where in the file the parser is currently looking.
	pos: Position,

	/// The last byte that was read.
	last_byte: u8,

	/// The next byte that will be read.
	/// 
	/// This is set to `Some` when `peek_byte` is called. When `read_byte` is called, it will first return this byte before reading any more from the reader.
	peeked_byte: Option<u8>,

	/// Initially `false`. Set to true upon reaching end-of-file.
	reached_eof: bool
}

impl<R: BufRead> Deserializer<R> {
	pub fn new(reader: R, file: Option<Rc<Path>>) -> Deserializer<R> {
		Deserializer {
			reader,
			pos: Position {
				file: file.into(),
				line: 1,
				column: 1
			},
			buf_b: Vec::with_capacity(4096),
			buf_s: String::with_capacity(4096),
			last_byte: 0,
			peeked_byte: None,
			reached_eof: false
		}
	}
}

pub fn from_reader<'de, T: Deserialize<'de>, R: BufRead>(reader: R, path: Option<Rc<Path>>) -> Result<T> {
	let mut deserializer = Deserializer::new(reader, path);
	let result = T::deserialize(&mut deserializer)?;
	Ok(result)
}

pub fn from_bytes<'de, T: Deserialize<'de>>(bytes: &[u8], file: Option<Rc<Path>>) -> Result<T> {
	from_reader(io::Cursor::new(bytes), file)
}

pub fn from_file<'de, T: Deserialize<'de>>(file: Rc<Path>) -> Result<T> {
	let file = file.into();

	match File::open(&file) {
		Ok(fh) => from_reader(BufReader::new(fh), Some(file)),
		Err(error) => Err(Error::Io { error, file: Some(file) })
	}
}
