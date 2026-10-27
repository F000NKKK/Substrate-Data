//! Every plain type of the schema system through every form: the value,
//! text, the straight binary form, a frame, and the shape's reading of
//! that frame — and the problems each one reports for a wrong value.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::sync::Arc;

use substrate_data::frame::{read_frame, read_shaped, to_frame};
use substrate_data::{
    Atom, KindName, OneOrList, Ordered, Ref, Schema, ShapeDoc, Uid, Value, from_text, read_bytes,
    read_value, to_bytes, to_text,
};

/// `x` out of every form it's written in, equal to itself.
fn through_every_form<T: Schema + PartialEq + Debug>(x: &T) {
    let value = x.write();
    assert_eq!(&read_value::<T>(&value).unwrap(), x, "value");
    let text = to_text(x);
    assert_eq!(&from_text::<T>(&text).unwrap().value, x, "text {text}");
    assert_eq!(&read_bytes::<T>(&to_bytes(x)).unwrap(), x, "binary");
    let frame = to_frame(x);
    assert_eq!(read_frame::<T>(&frame).as_ref(), Some(x), "frame");
    let seen = read_shaped(&frame, &T::shape());
    if let Some(seen) = seen {
        assert_eq!(&read_value::<T>(&seen).unwrap(), x, "shaped");
    }
    // Describing the shape never fails.
    let _ = ShapeDoc::of(&T::shape()).write();
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

fn problem_of<T: Schema + Debug>(v: Value) -> String {
    read_value::<T>(&v)
        .map(|x| panic!("read {x:?}"))
        .unwrap_err()
        .iter()
        .map(|d| d.message.clone())
        .collect::<Vec<_>>()
        .join("; ")
}

#[test]
fn a_wrong_value_is_a_problem_not_a_panic() {
    assert_eq!(
        problem_of::<bool>(Value::Int(1)),
        "expected true or false, found 1"
    );
    assert_eq!(
        problem_of::<String>(Value::Int(1)),
        "expected a string, found 1"
    );
    assert!(problem_of::<String>(Value::from("@file:x")).contains("expected plain text"));
    assert!(problem_of::<String>(Value::from("@nokind")).contains("a reference is `@kind:id`"));
    assert!(problem_of::<u8>(Value::Int(300)).contains("300 is out of range"));
    assert!(problem_of::<u8>(Value::Int(-1)).contains("out of range"));
    assert!(problem_of::<u8>(Value::from("x")).contains("expected an integer"));
    assert!(problem_of::<f32>(Value::from("x")).contains("expected a number"));
    assert!(problem_of::<Vec<u8>>(Value::Int(1)).contains("expected a list"));
    assert!(problem_of::<BTreeMap<String, u8>>(Value::Int(1)).contains("expected an object"));
    assert!(problem_of::<Ordered<u8>>(Value::Int(1)).contains("expected an object"));
    assert!(problem_of::<OneOrList<u8>>(Value::List(vec![])).contains("an empty list"));
    assert!(problem_of::<(u8, u8)>(Value::Int(1)).contains("a list of 2"));
    assert!(
        problem_of::<(u8, u8, u8)>(Value::List(vec![Value::Int(1)]))
            .contains("expected 3 items, found 1")
    );
    assert!(problem_of::<Result<u8, u8>>(Value::Int(1)).contains("{ ok } or { err }"));
    assert!(
        problem_of::<Result<u8, u8>>(Value::Map(Default::default())).contains("{ ok } or { err }")
    );
    assert!(problem_of::<Atom>(Value::Int(1)).contains("expected a string"));
    assert!(problem_of::<Uid>(Value::from("zz")).contains("not a valid uid"));
    assert!(problem_of::<Ref>(Value::from("plain")).contains("a reference `@kind:id`"));
    assert!(problem_of::<KindName>(Value::Int(1)).contains("a name"));
    assert!(problem_of::<KindName>(Value::from("Bad")).contains("isn't a kind"));
    let map = Value::Map([("a".to_owned(), Value::from("x"))].into_iter().collect());
    let problems = read_value::<BTreeMap<String, u8>>(&map).unwrap_err();
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
