use serde::de::{
	DeserializeSeed,
	MapAccess,
	IntoDeserializer,
	Visitor
};
use std::io::BufRead;
use super::{
	AaValueDeserializer,
	Deserializer,
	Error,
	FillBufResult,
	Result
};

impl<'de, R: BufRead> serde::Deserializer<'de> for &mut Deserializer<R> {
	type Error = Error;

	fn is_human_readable(&self) -> bool { true }

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		visitor.visit_map(AaTopMapAccess {
			de: self,
			no_value: false
		})
	}

	serde::forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
		bytes byte_buf option unit unit_struct newtype_struct seq tuple
		tuple_struct map struct enum identifier ignored_any
	}
}

struct AaTopMapAccess<'a, R: BufRead> {
	de: &'a mut Deserializer<R>,
	no_value: bool
}

impl<'de, 'a, R: BufRead> MapAccess<'de> for AaTopMapAccess<'a, R> {
	type Error = Error;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
	where K: DeserializeSeed<'de> {
		// Keys always occur at the beginning of a line, so if we're currently in the middle of a line, skip to the next line.
		if self.de.pos.column != 1 {
			loop {
				if let Some(byte) = self.de.read_byte()? {
					if byte == b'\r' || byte == b'\n' {
						// End of line.
						break
					}
				}
				else {
					// End of file.
					return Ok(None)
				}
			}
		}

		// Read the key, look for the delimiter, and prepare to submit the key to the `Visitor`.
		match self.de.fill_buf(&[b':'])? {
			FillBufResult::FoundDelim(_) => {
				// We've read in a key, and found the delimiter.
				self.no_value = false;
				
				// Before we proceed, we need to strip the space that (usually?) comes after the delimiter.
				match self.de.peek_byte()? {
					Some(b' ') => {
						// Found it. Now we need to consume it from the input so that it's not considered part of the value.
						// This can't fail and we don't need to see the byte again, so just throw away the result.
						let _ = self.de.read_byte();
					},
					_ => {
						// Found some other byte. Leave it; we'll consider it part of the value.
					}
				}
			},
			FillBufResult::FoundEof if self.de.buf_b.is_empty() => {
				// We've reached the end of the file and read nothing.
				return Ok(None)
			},
			_ => {
				// We've read a key with no value. We need to make note of this so that `next_value_seed` submits `()` instead of trying to read an actual value.
				self.no_value = true;
			}
		}

		// Keys are always strings, so decode it.
		self.de.decode_buf_all();

		// All ready. Submit the key to the `Visitor`.
		seed.deserialize((&self.de.buf_s[..]).into_deserializer()).map(Some)
	}

	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
	where V: DeserializeSeed<'de> {
		if self.no_value {
			// If we're at a key with no value, then say so.
			seed.deserialize(().into_deserializer())
		}
		else {
			// If there is a value, then pass a deserializer along to read it from.
			seed.deserialize(AaValueDeserializer::new(self.de))
		}
	}
}
