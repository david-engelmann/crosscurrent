//! The decimal number model the staging equality stands on
//! (XC-28).
//!
//! The Python implementation compares two values by reading
//! each with the runtime's decimal conversion, so the
//! grammar the comparison obeys is that conversion's own,
//! and it is wider than the ASCII worth grammar forkdrift
//! reads under its FD-19. The conversion strips the
//! runtime's whitespace set from both ends of the value and
//! removes every underscore character, then reads an
//! optional sign and an infinity word, a nan or snan word
//! with an optional digit payload, or a number over the
//! runtime's Unicode decimal digits with at most one dot
//! and an optional exponent. A finite number is worth
//! comparing only inside the exponent validity rules the
//! probes recorded, and two finite numbers are equal when
//! their canonical forms match. The digit string is never
//! rounded and never converted at bounded precision, so
//! the comparison is exact at any coefficient length.

use crate::pytext;

/// The lowest stored exponent a finite value may carry,
/// the written exponent less the fraction digit count.
const STORED_EXPONENT_FLOOR: i128 = -1_999_999_999_999_999_997;

/// The highest adjusted exponent a finite value may carry,
/// the stored exponent plus the coefficient digit count
/// less one.
const ADJUSTED_EXPONENT_CAP: i128 = 999_999_999_999_999_999;

/// The magnitude the written exponent accumulation clamps
/// at. Any exponent past this magnitude is out of range on
/// its own side under the two rules above whatever the
/// fraction digit count of a real text is, so the clamp
/// never changes a verdict.
const EXPONENT_CLAMP: i128 = 4_000_000_000_000_000_000;

/// The outcome of reading one value under the grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedNumber {
    /// The value matches no form the grammar admits, or a
    /// finite form outside the exponent validity rules.
    Invalid,
    /// The value is an infinity or a nan, which is never
    /// equal to anything.
    NonFinite,
    /// A finite value, held by its canonical form.
    Finite(NumberKey),
}

/// The canonical form of one finite value. Two finite
/// values are equal exactly when their keys are equal,
/// with the zero rule that an empty digit string is zero
/// whatever sign or exponent the spelling carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberKey {
    /// Whether the value is negative, never set for zero.
    pub negative: bool,
    /// The coefficient digits with leading and trailing
    /// zeros removed, as ASCII digits. Empty means zero.
    pub digits: String,
    /// The exponent the trimmed digits carry.
    pub exponent: i128,
}

impl NumberKey {
    /// Whether two canonical forms name the same worth.
    pub fn equals(&self, other: &NumberKey) -> bool {
        if self.digits.is_empty() || other.digits.is_empty() {
            return self.digits.is_empty() && other.digits.is_empty();
        }
        self.negative == other.negative
            && self.digits == other.digits
            && self.exponent == other.exponent
    }
}

/// The digit worth of a character in the runtime's decimal
/// digit set, or None when the character is not one. Every
/// range opens at the zero of its own alphabet, and the one
/// span that carries several alphabets in a row, the
/// mathematical alphanumeric digits, repeats the worths
/// zero to nine once per alphabet, so the worth is the
/// offset inside the range taken ten at a time.
fn digit_value(c: char) -> Option<u8> {
    let point = c as u32;
    for &(low, high) in pytext::DECIMAL_RANGES {
        if point >= low && point <= high {
            return Some(((point - low) % 10) as u8);
        }
    }
    None
}

/// Strip an ASCII keyword from the head of a text under
/// ASCII case-insensitive comparison, returning the rest.
/// The comparison runs on bytes, so a text whose head is
/// not ASCII simply never matches and no slice is taken
/// from it at a fixed byte offset.
fn strip_keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let word = keyword.as_bytes();
    if bytes.len() >= word.len()
        && bytes[..word.len()]
            .iter()
            .zip(word.iter())
            .all(|(byte, letter)| byte.eq_ignore_ascii_case(letter))
    {
        Some(&text[keyword.len()..])
    } else {
        None
    }
}

/// Whether a nan payload is admissible, empty or decimal
/// digits only. The payload is never observable through the
/// comparison, because a nan is never equal to anything and
/// an inadmissible payload only makes the value invalid,
/// which is never equal either.
fn payload_admissible(payload: &str) -> bool {
    payload.chars().all(|c| digit_value(c).is_some())
}

/// Read one value under the decimal grammar of XC-28.
pub fn parse_number(text: &str) -> ParsedNumber {
    let stripped = pytext::strip(text);
    let cleaned: String = stripped.chars().filter(|c| *c != '_').collect();
    let chars: Vec<char> = cleaned.chars().collect();
    let mut index = 0usize;
    let mut negative = false;
    if index < chars.len() && (chars[index] == '+' || chars[index] == '-') {
        negative = chars[index] == '-';
        index += 1;
    }
    let rest: String = chars[index..].iter().collect();
    if rest.eq_ignore_ascii_case("inf") || rest.eq_ignore_ascii_case("infinity") {
        return ParsedNumber::NonFinite;
    }
    for keyword in ["snan", "nan"] {
        if let Some(payload) = strip_keyword(&rest, keyword) {
            if payload_admissible(payload) {
                return ParsedNumber::NonFinite;
            }
            return ParsedNumber::Invalid;
        }
    }
    let mut coefficient: Vec<u8> = Vec::new();
    let mut fraction_digits = 0usize;
    let mut seen_dot = false;
    let mut in_exponent = false;
    let mut exponent_negative = false;
    let mut exponent_digits: Vec<u8> = Vec::new();
    let mut position = index;
    while position < chars.len() {
        let c = chars[position];
        if let Some(value) = digit_value(c) {
            if in_exponent {
                exponent_digits.push(value);
            } else {
                coefficient.push(value);
                if seen_dot {
                    fraction_digits += 1;
                }
            }
            position += 1;
            continue;
        }
        match c {
            '.' if !seen_dot && !in_exponent => {
                seen_dot = true;
                position += 1;
            }
            'e' | 'E' if !in_exponent => {
                in_exponent = true;
                position += 1;
                if position < chars.len() && (chars[position] == '+' || chars[position] == '-') {
                    exponent_negative = chars[position] == '-';
                    position += 1;
                }
            }
            _ => return ParsedNumber::Invalid,
        }
    }
    if coefficient.is_empty() {
        return ParsedNumber::Invalid;
    }
    if in_exponent && exponent_digits.is_empty() {
        return ParsedNumber::Invalid;
    }
    let mut written_exponent: i128 = 0;
    for value in &exponent_digits {
        written_exponent = written_exponent * 10 + i128::from(*value);
        if written_exponent > EXPONENT_CLAMP {
            written_exponent = EXPONENT_CLAMP;
        }
    }
    if exponent_negative {
        written_exponent = -written_exponent;
    }
    let stored_exponent = written_exponent - fraction_digits as i128;
    let leading_zeros = coefficient.iter().take_while(|value| **value == 0).count();
    let digit_count = if leading_zeros == coefficient.len() {
        1
    } else {
        (coefficient.len() - leading_zeros) as i128
    };
    let adjusted_exponent = stored_exponent + digit_count - 1;
    if stored_exponent < STORED_EXPONENT_FLOOR || adjusted_exponent > ADJUSTED_EXPONENT_CAP {
        return ParsedNumber::Invalid;
    }
    let first_nonzero = coefficient.iter().position(|value| *value != 0);
    let Some(first) = first_nonzero else {
        return ParsedNumber::Finite(NumberKey {
            negative: false,
            digits: String::new(),
            exponent: 0,
        });
    };
    let last = coefficient
        .iter()
        .rposition(|value| *value != 0)
        .expect("a nonzero digit exists");
    let trailing_removed = (coefficient.len() - 1 - last) as i128;
    let digits: String = coefficient[first..=last]
        .iter()
        .map(|value| (b'0' + value) as char)
        .collect();
    ParsedNumber::Finite(NumberKey {
        negative,
        digits,
        exponent: stored_exponent + trailing_removed,
    })
}
