//! Integers past their type's range, and 64-bit unsigned numbers past
//! `i64::MAX`, in every form.

use substrate_data::frame::{read_frame, to_frame};
use substrate_data::{Problem, Schema, Value, read_bytes, read_value, to_bytes};

fn is_out_of_range<T: Schema>(v: &Value) -> bool {
    let problems = read_value::<T>(v).err().expect("out of range");
    problems
        .iter()
        .all(|d| d.code.as_ref().is_some_and(|c| *c == Problem::OutOfRange))
}

#[test]
fn a_number_past_its_type_is_out_of_range() {
    assert!(is_out_of_range::<i8>(&Value::Int(300)));
    assert!(is_out_of_range::<u8>(&Value::Int(-1)));
    assert!(is_out_of_range::<u32>(&Value::Int(-5)));
}

#[test]
fn big_unsigned_numbers_round_trip_in_every_form() {
    let list: Vec<u64> = vec![1, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX];
    assert_eq!(read_value::<Vec<u64>>(&list.write()).as_ref(), Ok(&list));
    assert_eq!(read_bytes::<Vec<u64>>(&to_bytes(&list)).as_ref(), Ok(&list));
    assert_eq!(read_frame::<Vec<u64>>(&to_frame(&list)), Some(list));
}

#[test]
fn a_frame_list_of_big_unsigned_numbers_reads_as_the_values_write_makes() {
    let list: Vec<u64> = vec![1, u64::MAX, i64::MAX as u64 + 1];
    let frame = to_frame(&list);
    let value = substrate_data::frame::read_shaped(&frame, &Vec::<u64>::shape());
    assert_eq!(value, Some(list.write()));
}
