//! Cases for the scalar tags: null, bool, int, float and string.

use substrate_data::Value;

use crate::case::Case;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = vec![
        Case::new("null", "The null value: tag 0, no payload.").all_forms(Value::Null),
        Case::new("bool_false", "false: tag 1.").all_forms(Value::Bool(false)),
        Case::new("bool_true", "true: tag 2.").all_forms(Value::Bool(true)),
    ];
    all.extend(ints());
    all.extend(floats());
    all.extend(non_finite());
    all.extend(strings());
    all.extend(text_numbers());
    all
}

const INTS: [i64; 13] = [
    0,
    1,
    -1,
    63,
    64,
    -64,
    -65,
    8191,
    8192,
    2_147_483_647,
    2_147_483_648,
    i64::MAX,
    i64::MIN,
];

fn ints() -> Vec<Case> {
    INTS.iter()
        .map(|&n| {
            let name = if n < 0 {
                format!("int_minus_{}", n.unsigned_abs())
            } else {
                format!("int_{n}")
            };
            let zigzag = ((n << 1) ^ (n >> 63)).cast_unsigned();
            let case = Case::new(
                &name,
                &format!("The integer {n}: tag 3, then its zigzag form {zigzag} as a varint."),
            );
            case.all_forms(Value::Int(n))
        })
        .collect()
}

const FLOATS: [(&str, f64, &str); 12] = [
    ("zero", 0.0, "positive zero"),
    (
        "minus_zero",
        -0.0,
        "negative zero: its sign bit is kept in every form",
    ),
    (
        "one",
        1.0,
        "one, spelled 1.0 so the text reads back as a float",
    ),
    ("minus_one_and_a_half", -1.5, "a negative float"),
    (
        "tenth",
        0.1,
        "0.1, the shortest text that reads back as the same f64",
    ),
    (
        "half",
        0.5,
        "exactly an f32, still written as a full f64 (tag 4) in a plain value",
    ),
    (
        "ten_to_the_16",
        1e16,
        "text spells it 1e16, never 10000000000000000 (that reads as an int)",
    ),
    ("max", f64::MAX, "the largest finite f64"),
    ("min_positive", f64::MIN_POSITIVE, "the smallest normal f64"),
    ("subnormal", 5e-324, "the smallest subnormal f64"),
    ("small", 1e-7, "a small exponent"),
    (
        "f32_max",
        3.402_823_466_385_288_6e38,
        "the largest f32, as an f64",
    ),
];

fn floats() -> Vec<Case> {
    FLOATS
        .iter()
        .map(|&(name, f, why)| {
            let description =
                format!("A float, {why}: tag 4, then the f64 as 8 bytes little-endian.");
            Case::new(&format!("float_{name}"), &description).all_forms(Value::Float(f))
        })
        .collect()
}

fn non_finite() -> Vec<Case> {
    let note = "Binary only: the text form writes a non-finite float as null (a different value), \
                so no text is given. The binary form keeps the exact 64 bits, a NaN's payload and \
                sign included; compare floats by their bits.";
    let make = |name: &str, what: &str, bits: u64| {
        let value = Value::Float(f64::from_bits(bits));
        let bytes = substrate_data::binary::encode(&value);
        Case::new(name, &format!("{what}. {note}"))
            .value(value)
            .binary(bytes)
    };
    vec![
        make("float_infinity", "Positive infinity", 0x7ff0_0000_0000_0000),
        make(
            "float_minus_infinity",
            "Negative infinity",
            0xfff0_0000_0000_0000,
        ),
        make("float_nan", "The quiet NaN", 0x7ff8_0000_0000_0000),
        make(
            "float_nan_payload",
            "A NaN with a payload",
            0x7ff8_0000_0000_0001,
        ),
        make(
            "float_nan_negative",
            "A NaN with the sign bit set",
            0xfff8_0000_0000_0000,
        ),
    ]
}

const PLAIN_STRINGS: [(&str, &str, &str); 11] = [
    (
        "empty",
        "",
        "The empty string: a new table entry of length 0.",
    ),
    (
        "ascii",
        "hello",
        "A short ASCII string: tag 5, a new-string marker (length << 1 | 1), the bytes.",
    ),
    (
        "latin",
        "héllo wörld",
        "Accented letters: UTF-8, the length counts bytes (13), not characters.",
    ),
    ("cjk", "日本語", "Three characters, nine bytes."),
    (
        "emoji",
        "🦀 and 🧪",
        "Characters outside the BMP: four bytes each in UTF-8.",
    ),
    (
        "escapes",
        "quote \" backslash \\ newline \n tab \t",
        "Text escapes these; binary holds them raw.",
    ),
    (
        "control",
        "bell \u{7} and \u{1} and unit separator \u{1f}",
        "Control characters: text writes them as \\u00xx.",
    ),
    (
        "separators",
        "line \u{2028} and paragraph \u{2029} separators",
        "Not escaped in text, kept raw.",
    ),
    (
        "at_doubled",
        "@@everyone",
        "A string as spelled: a plain text that starts with `@` is stored with it doubled.",
    ),
    (
        "looks_like_ref",
        "@file:images/logo.png",
        "A string spelled like a reference is still a string value in the model; only a schema reads it as a ref.",
    ),
    (
        "looks_like_bytes",
        "@bytes:AQID",
        "A string spelled like bytes is still a string value in the model.",
    ),
];

fn strings() -> Vec<Case> {
    let long63 = "a".repeat(63);
    let long64 = "a".repeat(64);
    let mut out: Vec<Case> = PLAIN_STRINGS
        .iter()
        .map(|&(name, s, why)| Case::new(&format!("str_{name}"), why).all_forms(Value::from(s)))
        .collect();
    out.push(
        Case::new(
            "str_len_63",
            "63 bytes: the marker 127 is the last one-byte varint.",
        )
        .all_forms(Value::from(long63)),
    );
    out.push(
        Case::new("str_len_64", "64 bytes: the marker 129 takes two bytes.")
            .all_forms(Value::from(long64)),
    );
    out
}

fn text_numbers() -> Vec<Case> {
    let float = |name: &str, text: &str, f: f64, why: &str| {
        Case::new(name, why).text(text).value(Value::Float(f))
    };
    vec![
        float(
            "text_exponent_is_float",
            "1E5",
            100_000.0,
            "An exponent makes a float, even a whole one (the `E` may be upper case).",
        ),
        float(
            "text_exponent_two",
            "1e2",
            100.0,
            "1e2 is the float 100.0, not the int 100.",
        ),
        float(
            "text_int_past_i64",
            "9223372036854775808",
            9.223_372_036_854_776e18,
            "A whole number that does not fit i64 reads as a float.",
        ),
        float(
            "text_int_past_u64",
            "18446744073709551616",
            1.844_674_407_370_955_2e19,
            "Likewise far past u64.",
        ),
        float(
            "text_float_overflows",
            "1.5e400",
            f64::INFINITY,
            "A float too big for f64 reads as infinity (and writes back as null).",
        ),
        Case::new(
            "text_minus_zero_int",
            "`-0` without a fraction is the int 0.",
        )
        .text("-0")
        .value(Value::Int(0)),
        Case::new("text_int_min", "i64::MIN reads as an int.")
            .text("-9223372036854775808")
            .value(Value::Int(i64::MIN)),
    ]
}
