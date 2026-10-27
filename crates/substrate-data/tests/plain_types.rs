//! Every plain type of the schema system through every form: the value,
//! text, the straight binary form, a frame, and the shape's reading of
//! that frame — and the problems each one reports for a wrong value.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::sync::Arc;

use substrate_data::frame::{read_frame, read_shaped, to_frame};
use substrate_data::{
    Atom, KindName, Map, OneOrList, Ordered, Ref, Schema, ShapeDoc, Uid, Value, from_text,
    read_bytes, read_value, to_bytes, to_text,
};

/// `x` out of every form it's written in, equal to itself.
fn through_every_form<T: Schema + PartialEq + Debug>(x: &T) {
    let value = x.write();
    assert_eq!(&read_value::<T>(&value).expect("value"), x, "value");
    let text = to_text(x);
    // Written straight, the same text as through the value.
    assert_eq!(text, substrate_data::text::to_string_pretty(&value), "straight");
    assert_eq!(
        &from_text::<T>(&text).expect("text").value,
        x,
        "text {text}"
    );
    assert_eq!(&read_bytes::<T>(&to_bytes(x)).expect("binary"), x, "binary");
    let frame = to_frame(x);
    assert_eq!(read_frame::<T>(&frame).as_ref(), Some(x), "frame");
    let seen = read_shaped(&frame, &T::shape());
    if let Some(seen) = seen {
        assert_eq!(&read_value::<T>(&seen).expect("shaped"), x, "shaped");
    }
    // Describing the shape never fails.
    drop(ShapeDoc::of(&T::shape()).write());
}

#[test]
fn simple_values() {
    through_every_form(&true);
    through_every_form(&false);
    through_every_form(&());
    through_every_form(&Value::List(vec![Value::Int(1), Value::from("a")]));
    through_every_form(&String::from("plain"));
    through_every_form(&String::from("@@at"));
    through_every_form(&Cow::<'static, str>::Owned("cow".into()));
    through_every_form(&std::path::PathBuf::from("docs/a.txt"));
    through_every_form(&Atom::new("an_atom"));
    through_every_form(&Box::<str>::from("boxed"));
    through_every_form(&Arc::<str>::from("shared"));
    through_every_form(&Box::new(5u8));
    through_every_form(&Some(3i32));
    through_every_form(&None::<i32>);
    through_every_form(&Uid(0xabcd_u128 << 64 | 9));
    through_every_form(&Ref::parse("file", "a/b").unwrap());
    through_every_form(&KindName::new("doc").unwrap());
}

#[test]
fn numbers() {
    through_every_form(&-5i8);
    through_every_form(&200u8);
    through_every_form(&-300i16);
    through_every_form(&60_000u16);
    through_every_form(&-70_000i32);
    through_every_form(&4_000_000_000u32);
    through_every_form(&-5_000_000_000i64);
    through_every_form(&u64::MAX);
    through_every_form(&12usize);
    through_every_form(&1.5f32);
    through_every_form(&-2.25f64);
}

#[test]
fn collections() {
    through_every_form(&vec![1u32, 2, 3]);
    through_every_form(&vec![String::from("a"), String::from("b")]);
    through_every_form(&vec![1.5f32, 2.5]);
    through_every_form(&vec![0.5f64, 1.5]);
    through_every_form(&Vec::<u8>::new());
    through_every_form(&BTreeSet::from([3u32, 1, 2]));
    through_every_form(&BTreeSet::from([String::from("x")]));
    through_every_form(&[1u8, 2, 3]);
    through_every_form(&[0.5f32, 1.5]);
    through_every_form(&[String::from("a"), String::from("b")]);
    through_every_form(&Vec::<Box<[u8]>>::new());
    through_every_form(&Vec::from([1u8, 2]).into_boxed_slice());
    through_every_form(&BTreeMap::from([
        ("b".to_owned(), 2u32),
        ("a".to_owned(), 1),
    ]));
    through_every_form(&HashMap::from([("k".to_owned(), String::from("v"))]));
    through_every_form(&Ordered(vec![("z".to_owned(), 1u8), ("a".to_owned(), 2)]));
    through_every_form(&Ordered::<u8>::default());
    through_every_form(&OneOrList(vec![1u8]));
    through_every_form(&OneOrList(vec![1u8, 2]));
    through_every_form(&(1u8, String::from("a")));
    through_every_form(&(1u8, 2u16, 3u32));
    through_every_form(&(1u8, 2u16, 3u32, String::from("four")));
    through_every_form(&Ok::<u8, String>(1));
    through_every_form(&Err::<u8, String>("no".into()));
}

fn problem_of<T: Schema + Debug>(v: &Value) -> String {
    let problems = read_value::<T>(v).expect_err("a value that doesn't read");
    problems
        .iter()
        .map(|d| d.message.clone())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Reading `$value` as `$t` is a problem that says `$says`.
macro_rules! says {
    ($t:ty, $value:expr, $says:expr) => {
        let said = problem_of::<$t>(&$value);
        assert!(said.contains($says), "{said:?} doesn't say {:?}", $says);
    };
}

#[test]
fn a_value_of_the_wrong_case_is_a_problem_not_a_panic() {
    assert_eq!(
        problem_of::<bool>(&Value::Int(1)),
        "expected true or false, found 1"
    );
    assert_eq!(
        problem_of::<String>(&Value::Int(1)),
        "expected a string, found 1"
    );
    says!(u8, Value::from("x"), "expected an integer");
    says!(f32, Value::from("x"), "expected a number");
    says!(Vec<u8>, Value::Int(1), "expected a list");
    says!(BTreeMap<String, u8>, Value::Int(1), "expected an object");
    says!(Ordered<u8>, Value::Int(1), "expected an object");
    says!(Atom, Value::Int(1), "expected a string");
}

#[test]
fn text_that_is_not_plain_or_not_valid_is_a_problem() {
    says!(String, Value::from("@file:x"), "expected plain text");
    says!(String, Value::from("@nokind"), "a reference is `@kind:id`");
    says!(Uid, Value::from("zz"), "not a valid uid");
    says!(Ref, Value::from("plain"), "a reference `@kind:id`");
    says!(KindName, Value::Int(1), "a name");
    says!(KindName, Value::from("Bad"), "isn't a kind");
}

#[test]
fn numbers_out_of_range_and_lists_of_the_wrong_size_are_problems() {
    says!(u8, Value::Int(300), "300 is out of range");
    says!(u8, Value::Int(-1), "out of range");
    says!(OneOrList<u8>, Value::List(vec![]), "an empty list");
    says!((u8, u8), Value::Int(1), "a list of 2");
    says!(
        (u8, u8, u8),
        Value::List(vec![Value::Int(1)]),
        "expected 3 items, found 1"
    );
}

#[test]
fn a_result_is_an_ok_or_an_err_object() {
    says!(Result<u8, u8>, Value::Int(1), "{ ok } or { err }");
    says!(Result<u8, u8>, Value::Map(Map::new()), "{ ok } or { err }");
}

#[test]
fn a_problem_in_an_item_names_the_items_place() {
    let map = Value::Map([("a".to_owned(), Value::from("x"))].into_iter().collect());
    let problems = read_value::<BTreeMap<String, u8>>(&map).expect_err("a bad item");
    assert_eq!(problems[0].path.to_string(), "a");
}

#[test]
fn malformed_binary_and_frames_are_none() {
    let good = to_bytes(&vec![String::from("a"), String::from("b")]);
    for cut in 0..good.len() {
        assert!(
            read_bytes::<Vec<String>>(&good[..cut]).is_err(),
            "cut at {cut}"
        );
    }
    let frame = to_frame(&(1u8, String::from("a"), 2u32));
    for cut in 0..frame.len() {
        assert_eq!(
            read_frame::<(u8, String, u32)>(&frame[..cut]),
            None,
            "cut at {cut}"
        );
    }
    assert_eq!(read_frame::<Vec<u8>>(&[]), None);
    assert_eq!(read_frame::<Vec<u8>>(&[0xff; 12]), None);
    assert!(read_bytes::<u8>(&[0xff; 12]).is_err());
}
