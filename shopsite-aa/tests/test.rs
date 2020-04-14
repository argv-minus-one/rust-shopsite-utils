use serde::{Deserialize, Deserializer as _};
use shopsite_aa as aa;
use std::path::Path;

#[test]
fn test_main() {
	#[derive(Debug, Deserialize, Eq, PartialEq)]
	enum TestEnum {
		First,
		Second,
		Third
	}

	#[derive(Debug, Deserialize, Eq, PartialEq)]
	struct TestStruct {
		string: String,
		#[serde(rename = "“quoted”")] quoted: String,
		value_without_space: String,
		seq_empty1: Vec<String>,
		seq_empty2: Vec<String>,
		seq_one: Vec<String>,
		seq_multi: Vec<String>,
		seq_with_empty: Vec<String>,
		tuple: (String, u8, bool, serde_bytes::ByteBuf, char),
		r#enum: Vec<TestEnum>,
		some: Option<String>,
		none: Option<String>
	}

	let ts: TestStruct = aa::from_bytes(
		include_bytes!("test.aa"),
		Some(Path::new("test.aa").into())
	).unwrap();

	assert_eq!(ts.string, "string_value");
	assert_eq!(ts.quoted, "“value”");
	assert_eq!(ts.value_without_space, "Look ma, no space!");
	assert_eq!(ts.seq_empty1, Vec::<String>::new());
	assert_eq!(ts.seq_empty2, Vec::<String>::new());
	assert_eq!(ts.seq_one, vec!["Hello"]);
	assert_eq!(ts.seq_multi, vec!["Hello,", "world!"]);
	assert_eq!(ts.seq_with_empty, vec!["", "Hello,", "", "world!", ""]);
	assert_eq!(ts.tuple, ("Hello".to_string(), 42u8, true, serde_bytes::ByteBuf::from(b"world".to_vec()), '!'));
	assert_eq!(ts.r#enum, &[TestEnum::Third, TestEnum::First, TestEnum::Second]);
	assert_eq!(ts.some, Some("Hello".to_string()));
	assert_eq!(ts.none, None);
}

#[test]
fn test_no_final_eol() {
	// This test verifies that the parser doesn't choke when the end of a value is also the end of the file.

	#[derive(Debug, Eq, PartialEq, Deserialize)]
	struct TestWithNoFinalEol {
		value1: String,
		value2: String
	}

	let ts: TestWithNoFinalEol = aa::from_bytes(b"value1: Hello,\nvalue2: world!", None).unwrap();

	assert_eq!(ts.value1, "Hello,");
	assert_eq!(ts.value2, "world!");
}

#[test]
fn test_seq_variations() {
	// This test verifies that the parser doesn't choke when the end of the file occurs right after a sequence delimiter.

	#[derive(Debug, Eq, PartialEq, Deserialize)]
	struct TestSeq {
		seq: Vec<String>
	}

	// This would be a tad nicer if we had Scala's for-comprehensions…
	for comment_at_start in &[false, true] {
	for space_at_start in &[false, true] {
	for empty_elem_at_start in &[false, true] {
	for empty_elem_in_middle in &[false, true] {
	for empty_elem_at_end in &[false, true] {
	for comment_at_end in &[false, true] {
	for eol_at_end in &[false, true] {
		let mut input = Vec::<u8>::with_capacity(32);

		if *comment_at_start {
			input.extend_from_slice(b"#comment\n");
		}
		input.extend_from_slice(b"seq:");
		if *space_at_start {
			input.push(b' ');
		}
		if *empty_elem_at_start {
			input.push(b'|');
		}
		input.extend_from_slice(b"Hello|");
		if *empty_elem_in_middle {
			input.push(b'|');
		}
		input.extend_from_slice(b"world");
		if *empty_elem_at_end {
			input.push(b'|');
		}
		if *comment_at_end {
			input.extend_from_slice(b"\n#comment");
		}
		if *eol_at_end {
			input.push(b'\n');
		}

		let parsed: TestSeq = aa::from_bytes(&input[..], None).unwrap();

		let mut expected = Vec::<&'static str>::with_capacity(5);
		if *empty_elem_at_start {
			expected.push("");
		}
		expected.push("Hello");
		if *empty_elem_in_middle {
			expected.push("");
		}
		expected.push("world");
		if *empty_elem_at_end {
			expected.push("");
		}

		assert_eq!(parsed.seq, expected);
	}}}}}}}
}

#[test]
fn test_whitespace_lines_are_ignored() {
	// This test verifies that the parser doesn't interpret lines with only whitespace as significant.
	struct EmptyMapVisitor;
	impl<'de> serde::de::Visitor<'de> for EmptyMapVisitor {
		type Value = ();

		fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			write!(f, "an empty map")
		}

		fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
		where A: serde::de::MapAccess<'de> {
			let next_key: Option<String> = map.next_key()?;
			assert_eq!(next_key, None);
			Ok(())
		}
	}

	let mut deser = aa::Deserializer::new(std::io::Cursor::new(b" \n"), None);
	(&mut deser).deserialize_map(EmptyMapVisitor).unwrap();
}
