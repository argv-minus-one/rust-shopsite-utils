use serde::de::{
	DeserializeSeed,
	IntoDeserializer,
	SeqAccess,
	Visitor
};
use std::{
	io::BufRead,
	str::FromStr
};
use super::{
	Deserializer,
	Error,
	FillBufResult,
	Result
};

macro_rules! deserialize_with_other {
	($deserialize_from:ident, $deserialize_to:ident) => {
		fn $deserialize_from<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
			self.$deserialize_to(visitor)
		}
	}
}

macro_rules! deserialize_with_from_str {
	($deserialize_name:ident, $visit_name:ident, $error_kind:ident) => {
		fn $deserialize_name<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
			let start_pos = self.de.pos.clone();
			self.fill_buf_auto()?;
			self.de.decode_buf_all();
			visitor.$visit_name (
				FromStr::from_str(&self.de.buf_s[..])
				.map_err(|error| Error::$error_kind { error: error, pos: start_pos })?
			)
		}
	}
}

pub(super) struct AaValueDeserializer<'a, R: BufRead> {
	de: &'a mut Deserializer<R>,

	/// `true` iff the value being deserialized is inside of a sequence.
	/// 
	/// Elements in a sequence are delimited by `|` characters, so if this is `true`, then reading will only proceed up to the next such delimiter, rather than reading all the way to the end of the line.
	inside_seq: bool
}

impl<'a, R: BufRead> AaValueDeserializer<'a, R> {
	#[inline]
	pub(super) fn new(de: &'a mut Deserializer<R>) -> AaValueDeserializer<'a, R> {
		AaValueDeserializer {
			de,
			inside_seq: false
		}
	}
}

impl<'a, R: BufRead> AaValueDeserializer<'a, R> {
	/// Same effect as `self.de.fill_buf`, but with the delimiters automatically filled in with `self.read_until`.
	fn fill_buf_auto(&mut self) -> Result<FillBufResult> {
		self.de.fill_buf(match self.inside_seq {
			true => &[b'|'],
			false => &[]
		})
	}
}

impl<'de, 'a, R: BufRead> serde::Deserializer<'de> for AaValueDeserializer<'a, R> {
	type Error = Error;

	fn is_human_readable(&self) -> bool { true }

	fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.fill_buf_auto()?;
		visitor.visit_bytes(&self.de.buf_b[..])
	}

	fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.fill_buf_auto()?;
		self.de.decode_buf_all();
		visitor.visit_str(&self.de.buf_s[..])
	}

	fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.fill_buf_auto()?;

		// The recipient wants the text decoded, but wants to own the decoded `String`. Can do!
		visitor.visit_string(self.de.decode_buf_all_owned())
	}

	fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.fill_buf_auto()?;
		self.de.decode_buf_all();
		let mut chars = self.de.buf_s.chars();

		match (chars.next(), chars.next()) {
			(Some(only_char), None) => {
				// Success. The value is exactly one character long, just as requested.
				visitor.visit_char(only_char)
			},
			_ => {
				// Failure. The value is more than one character long, or is empty. Supply it as a string.
				visitor.visit_str(&self.de.buf_s[..])
			}
		}
	}

	fn deserialize_unit_struct<V>(self, _: &'static str, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.deserialize_unit(visitor)
	}

	fn deserialize_unit<V>(mut self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.fill_buf_auto()?;

		if self.de.buf_b.is_empty() {
			// The value here is empty, which is as close to a concept of “null” or “no value” as this format has.
			visitor.visit_unit()
		}
		else {
			// It's not empty. Deliver the bad news.
			self.deserialize_any(visitor)
		}
	}

	fn deserialize_newtype_struct<V>(self, _: &'static str, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		// Yeah, sure, buddy, we got your fancy “newtype struct” in this here dead-simple key-value format. Uh huh. Whatever you say, boss.
		visitor.visit_newtype_struct(self)
	}

	fn deserialize_tuple_struct<V>(self, _: &'static str, _: usize, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		// What format do you think this is? RON?
		self.deserialize_seq(visitor)
	}

	fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.deserialize_seq(visitor)
	}

	fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		// You're going to just ignore whatever I give you? Uh, ok. In that case, I'll give you nothing. Save ourselves both some time.
		visitor.visit_unit()
	}

	fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		// In this case, we'll consider an empty value to mean `None` and a non-empty value to mean `Some`.
		match self.de.peek_byte()? {
			None | Some(b'\r') | Some(b'\n') => {
				// The next byte is a line ending or end-of-file. That's a `None` for our purposes.
				visitor.visit_none()
			},
			Some(_) => {
				// The next byte is something else. That's a `Some`.
				visitor.visit_some(self)
			}
		}
	}

	fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		visitor.visit_seq(AaValueSeqAccess {
			de: self.de,
			is_first_element: true,
			is_nested_seq: self.inside_seq
		})
	}

	fn deserialize_enum<V>(mut self, _: &'static str, _: &'static [&'static str], visitor: V) -> Result<V::Value>
	where V: Visitor<'de> {
		self.fill_buf_auto()?;
		self.de.decode_buf_all();
		visitor.visit_enum((&self.de.buf_s[..]).into_deserializer())
	}

	deserialize_with_from_str!(deserialize_bool, visit_bool, InvalidBool);
	deserialize_with_from_str!(deserialize_i8, visit_i8, InvalidInt);
	deserialize_with_from_str!(deserialize_i16, visit_i16, InvalidInt);
	deserialize_with_from_str!(deserialize_i32, visit_i32, InvalidInt);
	deserialize_with_from_str!(deserialize_i64, visit_i64, InvalidInt);
	deserialize_with_from_str!(deserialize_i128, visit_i128, InvalidInt);
	deserialize_with_from_str!(deserialize_u8, visit_u8, InvalidInt);
	deserialize_with_from_str!(deserialize_u16, visit_u16, InvalidInt);
	deserialize_with_from_str!(deserialize_u32, visit_u32, InvalidInt);
	deserialize_with_from_str!(deserialize_u64, visit_u64, InvalidInt);
	deserialize_with_from_str!(deserialize_u128, visit_u128, InvalidInt);
	deserialize_with_from_str!(deserialize_f32, visit_f32, InvalidFloat);
	deserialize_with_from_str!(deserialize_f64, visit_f64, InvalidFloat);
	deserialize_with_other!(deserialize_byte_buf, deserialize_bytes);
	deserialize_with_other!(deserialize_any, deserialize_str);

	serde::forward_to_deserialize_any! {
		map struct identifier
	}
}

/// Accessor for a sequence of values.
/// 
/// In the ShopSite `.aa` format, items in a sequence are separated by a `|` (pipe) character.
struct AaValueSeqAccess<'a, R: BufRead> {
	de: &'a mut Deserializer<R>,

	/// Initially `true`. Set to `false` just before `next_element_seed` returns.
	is_first_element: bool,

	/// `true` if this is a nested sequence. Nested sequences have only one element.
	is_nested_seq: bool
}

impl<'de, 'a, R: BufRead> SeqAccess<'de> for AaValueSeqAccess<'a, R> {
	type Error = Error;

	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
	where T: DeserializeSeed<'de> {
		if
			// Nested sequences have only one element.
			(self.is_nested_seq && !self.is_first_element) ||
			// We've reached the end of the sequence.
			self.de.pos.column == 1 || self.de.reached_eof ||
			// This is an empty sequence. That is, this is the first element, and the next call to `read_byte` will yield either end-of-file or a line ending.
			(self.is_first_element && self.de.peek_byte()?.filter(|b| *b != b'\r' && *b != b'\n').is_none())
		{
			Ok(None)
		}
		else {
			// There's another element in the sequence, so let's pass it along.
			let ret = seed.deserialize(AaValueDeserializer {
				de: self.de,
				inside_seq: true
			}).map(Some);
			self.is_first_element = false;
			ret
		}
	}
}
