use encoding::{
	all::WINDOWS_1252,
	types::{DecoderTrap, Encoding}
};
use std::{
	io::{self, BufRead},
	slice::{self, SliceIndex}
};
use super::{
	Error,
	Deserializer,
	Result
};

/// Outcome of `Deserializer::fill_buf` (aside from I/O errors).
pub(super) enum FillBufResult {
	/// One of the delimiters was found. Contains the delimiter that was found.
	FoundDelim(u8),

	/// No delimiter was found before the end of the line.
	FoundEol,

	/// No delimiter was found before the end of the file.
	FoundEof
}

impl<R: BufRead> Deserializer<R> {
	/// Reads the next byte of input, keeping track of row and column numbers.
	pub(super) fn read_byte(&mut self) -> Result<Option<u8>> {
		// If we've already reached the end of the file, don't bother trying to read more.
		if self.reached_eof {
			return Ok(None);
		}

		// Either get the byte that was peeked, or read a new one.
		let read_result = {
			if let Some(peeked_byte) = self.peeked_byte {
				// If a byte was peeked, take that byte instead.
				self.peeked_byte = None;
				Some(peeked_byte)
			}
			else {
				// If no byte was peeked, then actually read a new byte.
				self.read_byte_raw()?
			}
		};

		// If `read_result` is `None`, then we've reached the end of the file. If not…
		if let Some(byte) = read_result {
			// Keep track of line and column numbers.
			match (self.last_byte, byte) {
				(b'\r', b'\n') => {
					// Don't increment the line number for the LF in a CR+LF pair. Treat these as one line break, not two.
				},
				(_, b'\r') | (_, b'\n') => {
					// New line. Increment the line number and reset the column number.
					self.pos.line += 1;
					self.pos.column = 1;
				},
				(_, b'\t') => {
					// Tabs increment the column number by 8 instead of 1.
					self.pos.column += 8;
				},
				(_, 0..=31) | (_, 127) => {
					// Control codes and DEL have zero width.
					// Backspaces arguably have *negative* width, but computers (unlike telegraphs) don't generally interpret them that way, so nah.
					// We are not keeping track of ANSI escape sequences. F#@% that.
				},
				_ => {
					// Everything else increments the column number by 1.
					self.pos.column += 1;
				}
			}

			// Record this as the last byte.
			self.last_byte = byte;
		}
		else {
			// We've reached the end of the file. Take note of this.
			self.reached_eof = true;
			self.last_byte = 0;
		}

		// Return the result of the read.
		Ok(read_result)
	}

	/// Gets what will be the next byte returned by `read_byte`, but without moving the “cursor”.
	pub(super) fn peek_byte(&mut self) -> Result<Option<u8>> {
		// If we've already reached the end of the file, don't bother trying to read more.
		if self.reached_eof {
			Ok(None)
		}
		else if let Some(peeked_byte) = self.peeked_byte {
			// If this function has already been called without actually reading the peeked byte, then return the same peeked byte again.
			Ok(Some(peeked_byte))
		}
		else {
			// Otherwise, read a new byte, but store it as the peeked byte.
			let byte_opt = self.read_byte_raw()?;
			self.peeked_byte = byte_opt;
			Ok(byte_opt)
		}
	}

	/// Reads a byte from the reader. Retries when interrupted. Does not respect peeking or track line and column numbers. Called by `peek_byte` and `read_byte`.
	fn read_byte_raw(&mut self) -> Result<Option<u8>> {
		let mut byte = 0u8;

		loop {
			return match self.reader.read(slice::from_mut(&mut byte)) {
				Ok(0) => {
					// If the reader read 0 bytes, then this is the end of the file. Return accordingly.
					Ok(None)
				},
				Ok(_) => {
					// Read a byte.
					Ok(Some(byte))
				},
				Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
					// Operation was interrupted. Keep trying.
					continue
				},
				Err(error) => {
					// I/O error!
					Err(Error::Io {
						error,
						file: self.pos.file.clone()
					})
				}
			}
		}
	}

	/// Clears `self.buf_b`, then fills it with input until reaching one of the given delimiter bytes, the end of the line, or the end of the file.
	/// 
	/// The `delimiters` may be an empty slice, in which case this method will simply read to the end of the line or file. If `delimiters` is not empty, then each byte read will be compared with each byte in `delimiters`, and reading ends when a match is found.
	/// 
	/// The buffer will not contain the delimiter or end-of-line marker. Blank lines and comment lines are skipped over.
	/// 
	/// If called at the beginning of a line, this will skip comment lines, blank lines, and lines with only whitespace. If called in the middle of reading a line, comments are not recognized and whitespace is not ignored.
	/// 
	/// The return value indicates the outcome of the operation, including which delimiter was found (if any).
	/// 
	/// # Errors
	/// 
	/// This method may fail with a `std::io::Error`. Calling it again after such a failure may have bogus results.
	pub(super) fn fill_buf(&mut self, delimiters: &[u8]) -> Result<FillBufResult> {
		self.buf_b.clear();

		let mut in_comment = false;
		let mut seen_non_whitespace = false;

		// If this function starts from the beginning of a line, then `self.pos.column` will be 1, either because the previous call to this function found a line ending or because this is the beginning of the file.
		let started_at_start_of_line = self.pos.column == 1;

		loop {
			// Which column are we reading from?
			let prev_column = self.pos.column;

			// OK, read the next byte.
			if let Some(byte) = self.read_byte()? {
				if byte == b'#' && (prev_column == 1 || (started_at_start_of_line && !seen_non_whitespace)) {
					// This is the beginning of a comment line.
					// Comment lines start with a `#` character, possibly after whitespace. `#` characters after non-whitespace characters do not count as comments. For example, on the line `bgcolor: #FFFFD6`, the key is `bgcolor` and the value is `#FFFFD6`.
					in_comment = true;

					// Clear the buffer, in case the comment begins after some whitespace.
					self.buf_b.clear();
				}
				else if in_comment && byte != b'\r' && byte != b'\n' {
					// We're still inside a comment line. Skip this byte.
				}
				else if byte == b'\r' || byte == b'\n' {
					// This is a line ending. Where is it?
					if in_comment {
						// It's the end of a comment line. We're out of the comment line now, but still haven't seen any significant text yet.
						in_comment = false;
					}
					else if prev_column == 1 {
						// It's the end of an empty line or part of a CR+LF sequence. Ignore it and keep going.
					}
					else if started_at_start_of_line && !seen_non_whitespace {
						// It's the end of a line containing only whitespace. Clear the buffer and skip to the next line, then.
						// This can only be the case if we started at the beginning of a line. If this function is called in the *middle* of a line, then what we're looking at is an empty or all-whitespace *value*, which is not the same thing and is treated as significant.
						self.buf_b.clear();
					}
					else {
						// By process of elimination, this must be the end of a line that isn't a comment, empty, or all whitespace. That means we're done filling the buffer, but didn't find a delimiter.
						return Ok(FillBufResult::FoundEol)
					}
				}
				else if delimiters.contains(&byte) {
					// Found a delimiter!
					return Ok(FillBufResult::FoundDelim(byte))
				}
				else {
					// Not a delimiter or a line ending. Add it to the buffer, and take note if it's not whitespace. Then keep looking.
					self.buf_b.push(byte);

					if !byte.is_ascii_whitespace() {
						seen_non_whitespace = true;
					}
				}
			}
			else {
				// If there are no more bytes to read, then we've reached the end of the file.
				// If we never saw any non-whitespace, then the last line is effectively blank, so clear the buffer of any whitespace left in it.
				if !seen_non_whitespace {
					self.buf_b.clear();
				}

				return Ok(FillBufResult::FoundEof)
			}
		}
	}

	/// Clears `self.buf_s`, then decodes part of `self.buf_b` into it.
	/// 
	/// Windows-1252 cannot fail to decode, so this method does not return a `Result`. It always succeeds (or panics).
	/// 
	/// # Panics
	/// 
	/// If the given `range` is out of bounds, this method will likely panic.
	pub(super) fn decode_buf(&mut self, range: impl SliceIndex<[u8], Output=[u8]>) {
		self.buf_s.clear();

		// The infallibility of Windows-1252 decoding is verified by a unit test, below.
		WINDOWS_1252.decode_to(&self.buf_b[range], DecoderTrap::Replace, &mut self.buf_s).unwrap();
	}

	/// Clears `self.buf_s`, then decodes all of `self.buf_b` into it.
	/// 
	/// Windows-1252 cannot fail to decode, so this method does not return a `Result`. It always succeeds.
	pub(super) fn decode_buf_all(&mut self) {
		self.decode_buf(..)
	}

	/// Decodes part of `self.buf_b` into a new `String`.
	/// 
	/// Windows-1252 cannot fail to decode, so this method does not return a `Result`. It always succeeds (or panics).
	/// 
	/// # Panics
	/// 
	/// If the given `range` is out of bounds, this method will likely panic.
	pub(super) fn decode_buf_owned(&mut self, range: impl SliceIndex<[u8], Output=[u8]>) -> String {
		WINDOWS_1252.decode(&self.buf_b[range], DecoderTrap::Replace).unwrap()
	}

	/// Decodes all of `self.buf_b` into a new `String`.
	/// 
	/// Windows-1252 cannot fail to decode, so this method does not return a `Result`. It always succeeds (or panics).
	pub(super) fn decode_buf_all_owned(&mut self) -> String {
		self.decode_buf_owned(..)
	}
}

#[test]
fn test_decoding_windows_1252_cannot_fail() {
	// We assume above that decoding Windows-1252 can never fail. This verifies that that's actually true by throwing every single Windows-1252 code point at the decoder.

	// Assemble the byte array.
	let mut bytes = [0u8; 256];
	for i in 0u8..=255u8 {
		bytes[i as usize] = i;
	}
	
	// Check that we assembled the byte array correctly.
	assert_eq!(bytes[0], 0u8);
	assert_eq!(bytes[127], 127u8);
	assert_eq!(bytes[255], 255u8);

	// Now, throw it at the decoder and make sure it doesn't fail. The decoder's output doesn't actually matter here, just that it succeeds.
	WINDOWS_1252.decode(&bytes[..], DecoderTrap::Replace).expect("Decoding Windows-1252 should never fail!");
}
