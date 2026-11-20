//! The text transformations shown as buttons in the UI.
//!
//! To add a new transformation:
//! 1. add a variant to [`Transform`],
//! 2. list it in [`Transform::GROUPS`],
//! 3. give it a label and a name,
//! 4. handle it in [`Transform::apply`].

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine as _;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
	Base64Encode,
	Base64Decode,
	UrlEncode,
	UrlDecode,
	HexEncode,
	HexDecode,
	Uppercase,
	Lowercase,
	Reverse,
}

impl Transform {
	/// Groups of buttons, rendered as one column each in the UI.
	pub const GROUPS: &'static [(&'static str, &'static [Transform])] = &[
		("Base64", &[Self::Base64Encode, Self::Base64Decode]),
		("URL", &[Self::UrlEncode, Self::UrlDecode]),
		("Hex", &[Self::HexEncode, Self::HexDecode]),
		("Text", &[Self::Uppercase, Self::Lowercase, Self::Reverse]),
	];

	/// Short text shown on the button. The group label supplies the context.
	pub fn label(self) -> &'static str {
		match self {
			Self::Base64Encode | Self::UrlEncode | Self::HexEncode => "Encode",
			Self::Base64Decode | Self::UrlDecode | Self::HexDecode => "Decode",
			Self::Uppercase => "UPPERCASE",
			Self::Lowercase => "lowercase",
			Self::Reverse => "Reverse",
		}
	}

	/// Full name used in status messages.
	pub fn name(self) -> &'static str {
		match self {
			Self::Base64Encode => "Base64 encode",
			Self::Base64Decode => "Base64 decode",
			Self::UrlEncode => "URL encode",
			Self::UrlDecode => "URL decode",
			Self::HexEncode => "Hex encode",
			Self::HexDecode => "Hex decode",
			Self::Uppercase => "Uppercase",
			Self::Lowercase => "Lowercase",
			Self::Reverse => "Reverse",
		}
	}

	/// Transform `input`, returning either the result or a human-readable error.
	pub fn apply(self, input: &str) -> Result<String, String> {
		match self {
			Self::Base64Encode => Ok(STANDARD.encode(input.as_bytes())),
			Self::Base64Decode => decode_base64(input),
			Self::UrlEncode => Ok(utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()),
			Self::UrlDecode => (
				percent_decode_str(input)
				. decode_utf8()
				. map(|decoded| decoded.into_owned())
				. map_err(|err| format!("invalid percent-encoding: {err}"))
			),
			Self::HexEncode => Ok(encode_hex(input)),
			Self::HexDecode => decode_hex(input),
			Self::Uppercase => Ok(input.to_uppercase()),
			Self::Lowercase => Ok(input.to_lowercase()),
			Self::Reverse => Ok(input.chars().rev().collect()),
		}
	}
}

fn decode_base64(input: &str) -> Result<String, String> {
	// Be forgiving about line breaks and other whitespace in pasted data.
	let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();

	let bytes = (
		STANDARD
		. decode(&compact)
		. or_else(|_| STANDARD_NO_PAD.decode(&compact))
		. or_else(|_| URL_SAFE.decode(&compact))
		. or_else(|_| URL_SAFE_NO_PAD.decode(&compact))
		. map_err(|err| format!("invalid base64: {err}"))?
	);

	String::from_utf8(bytes).map_err(|_| "decoded bytes are not valid UTF-8 text".to_owned())
}

fn encode_hex(input: &str) -> String {
	use std::fmt::Write as _;

	let mut out = String::with_capacity(input.len() * 2);
	for byte in input.bytes() {
		let _ = write!(out, "{byte:02x}");
	}
	out
}

fn decode_hex(input: &str) -> Result<String, String> {
	let mut nibbles = Vec::with_capacity(input.len());
	for character in input.chars() {
		if character.is_whitespace() {
			continue;
		}
		let value = (
			character
			. to_digit(16)
			. ok_or_else(|| format!("invalid hex character {character:?}"))?
		);
		nibbles.push(value as u8);
	}

	if nibbles.len() % 2 != 0 {
		return Err("hex input has an odd number of digits".to_owned());
	}

	let bytes: Vec<u8> = (
		nibbles
		. chunks_exact(2)
		. map(|pair| (pair[0] << 4) | pair[1])
		. collect()
	);

	String::from_utf8(bytes).map_err(|_| "decoded bytes are not valid UTF-8 text".to_owned())
}

#[cfg(test)]
mod tests {
	use super::Transform;

	fn roundtrip(encode: Transform, decode: Transform, text: &str) {
		let encoded = encode.apply(text).expect("encoding should succeed");
		let decoded = decode.apply(&encoded).expect("decoding should succeed");
		assert_eq!(decoded, text);
	}

	#[test]
	fn base64_known_vector() {
		assert_eq!(Transform::Base64Encode.apply("hello").unwrap(), "aGVsbG8=");
		assert_eq!(Transform::Base64Decode.apply("aGVsbG8=").unwrap(), "hello");
	}

	#[test]
	fn base64_roundtrips_unicode() {
		roundtrip(
			Transform::Base64Encode,
			Transform::Base64Decode,
			"héllo 世界 🦀\nsecond line",
		);
	}

	#[test]
	fn base64_decode_ignores_whitespace() {
		assert_eq!(
			Transform::Base64Decode.apply("aGVs\nbG8=").unwrap(),
			"hello",
		);
	}

	#[test]
	fn base64_rejects_garbage() {
		assert!(Transform::Base64Decode
		. apply("definitely not base64!!")
		. is_err());
	}

	#[test]
	fn url_roundtrips_special_characters() {
		roundtrip(
			Transform::UrlEncode,
			Transform::UrlDecode,
			"a b&c=d/é?x=1#frag",
		);
	}

	#[test]
	fn hex_roundtrips_unicode() {
		roundtrip(Transform::HexEncode, Transform::HexDecode, "Hello, 世界!");
	}

	#[test]
	fn hex_decode_accepts_uppercase_and_spaces() {
		assert_eq!(
			Transform::HexDecode.apply("48 65 6C 6C 6F").unwrap(),
			"Hello",
		);
	}

	#[test]
	fn hex_decode_rejects_odd_length() {
		assert!(Transform::HexDecode.apply("abc").is_err());
	}

	#[test]
	fn uppercase_and_lowercase() {
		assert_eq!(Transform::Uppercase.apply("Straße").unwrap(), "STRASSE");
		assert_eq!(Transform::Lowercase.apply("ÄBC").unwrap(), "äbc");
	}

	#[test]
	fn reverse_handles_unicode() {
		assert_eq!(Transform::Reverse.apply("abc🦀").unwrap(), "🦀cba");
	}
}
