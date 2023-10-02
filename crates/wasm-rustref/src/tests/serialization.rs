use crate::serialization::{deserialize_clarity_value, serialize_clarity_value};
use clarity::vm::{
    types::{
        ASCIIData, BuffData, CharType, ListData, ListTypeData, OptionalData, ResponseData,
        SequenceData, TypeSignature, UTF8Data,
    },
    Value,
};

#[test]
fn test_serialize_uint() {
    let value = Value::UInt(u128::MAX);

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    assert_eq!(value.expect_u128(), deserialized.expect_u128());
}

#[test]
fn test_serialize_int() {
    let value = Value::Int(i128::MAX);

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    assert_eq!(value.expect_i128(), deserialized.expect_i128());
}

#[test]
fn test_serialize_bool() {
    let value = Value::Bool(true);

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    assert_eq!(value.expect_bool(), deserialized.expect_bool());
}

#[test]
fn test_serialize_optional_none() {
    let value = Value::Optional(OptionalData { data: None });

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    assert_eq!(None, deserialized.expect_optional());
}

#[test]
fn test_serialize_optional_some() {
    let value = Value::Optional(OptionalData {
        data: Some(Box::new(Value::Int(5))),
    });

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    let val = deserialized.expect_optional();
    assert!(val.is_some());

    if let Some(deserialized_value) = val {
        assert_eq!(Value::Int(5), deserialized_value);
    }
}

#[test]
fn test_serialize_response_ok() {
    let value = Value::Response(ResponseData {
        committed: true,
        data: Box::new(Value::Int(5)),
    });

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    let response_value = deserialized.expect_result_ok();
    assert_eq!(Value::Int(5), response_value);
}

#[test]
fn test_serialize_response_err() {
    let value = Value::Response(ResponseData {
        committed: false,
        data: Box::new(Value::Int(5)),
    });

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    let response_value = deserialized.expect_result_err();
    assert_eq!(Value::Int(5), response_value);
}

#[test]
fn test_serialize_buffer() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let value = Value::Sequence(SequenceData::Buffer(BuffData { data: data.clone() }));

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    let val = deserialized.expect_buff(data.len());

    assert_eq!(&data, &val);
}

#[test]
fn test_serialize_ascii_string() {
    let data = "hello world!";
    let value = Value::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
        data: data.as_bytes().to_vec(),
    })));

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    let val = deserialized.expect_ascii();
    assert_eq!(data, &val);
}

#[test]
fn test_serialize_utf8_string() {
    let str = "hello world!";
    let mut data = Vec::<Vec<u8>>::new();
    for chunk in str.as_bytes().chunks(4) {
        data.push(vec![chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    let value = Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data { data })));

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    assert_eq!(value, deserialized);
}

#[test]
fn test_serialize_list_of_ints() {
    let value = Value::Sequence(SequenceData::List(ListData {
        data: vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
            Value::Int(5),
        ],
        type_signature: ListTypeData::new_list(TypeSignature::IntType, 5)
            .expect("Could not construct list"),
    }));

    let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

    println!("serialized: {:?}", serialized);

    assert!(
        serialized.len() > 0,
        "Expected serialized bytes to be longer than 0"
    );

    let deserialized = deserialize_clarity_value(&serialized).expect("Failed to deserialize value");

    assert_eq!(value, deserialized);
}
