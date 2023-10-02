use clarity::vm::{
    types::{
        ASCIIData, BuffData, CharType, ListData, ListTypeData, OptionalData, ResponseData,
        SequenceData, TypeSignature, UTF8Data,
    },
    Value,
};
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_rustref::serialization::*;

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

pub fn criterion_benchmark(c: &mut Criterion) {
    // ================================================================================
    // `uint` serialization
    // ================================================================================
    let mut uint_group = c.benchmark_group("uint");

    uint_group.bench_function("uint-serialize", |b| {
        let value = Value::UInt(u128::MAX);

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    uint_group.bench_function("uint-deserialize", |b| {
        let value = Value::UInt(u128::MAX);
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    uint_group.finish();

    // ================================================================================
    // `int` serialization
    // ================================================================================
    let mut int_group = c.benchmark_group("int");

    int_group.bench_function("int-serialize", |b| {
        let value = Value::Int(i128::MAX);

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    int_group.bench_function("int-deserialize", |b| {
        let value = Value::Int(i128::MAX);

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    int_group.finish();

    // ================================================================================
    // `bool` serialization
    // ================================================================================
    let mut bool_group = c.benchmark_group("bool");

    bool_group.bench_function("bool-serialize", |b| {
        let value = Value::Bool(true);

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    bool_group.bench_function("bool-deserialize", |b| {
        let value = Value::Bool(true);
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    bool_group.finish();

    // ================================================================================
    // `string-ascii` serialization
    // ================================================================================
    let mut string_ascii_group = c.benchmark_group("string-ascii");

    string_ascii_group.bench_function("string-ascii-serialize", |b| {
        let value = Value::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
            data: "hello world!".as_bytes().to_vec(),
        })));

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    string_ascii_group.bench_function("string-ascii-deserialize", |b| {
        let value = Value::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
            data: "hello world!".as_bytes().to_vec(),
        })));
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    string_ascii_group.finish();

    // ================================================================================
    // `string-utf8` serialization
    // ================================================================================
    let mut string_utf8_group = c.benchmark_group("string-utf8");

    string_utf8_group.bench_function("string-utf8-serialize", |b| {
        let str = "hello world!";
        let mut data = Vec::<Vec<u8>>::new();
        for chunk in str.as_bytes().chunks(4) {
            data.push(vec![chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        let value = Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data { data })));

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    string_utf8_group.bench_function("string-utf8-deserialize", |b| {
        let str = "hello world!";
        let mut data = Vec::<Vec<u8>>::new();
        for chunk in str.as_bytes().chunks(4) {
            data.push(vec![chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        let value = Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data { data })));
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    string_utf8_group.finish();

    // ================================================================================
    // `buffer` serialization
    // ================================================================================
    let mut buffer_group = c.benchmark_group("buffer");

    buffer_group.bench_function("buffer-serialize", |b| {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let value = Value::Sequence(SequenceData::Buffer(BuffData { data: data.clone() }));

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    buffer_group.bench_function("buffer-deserialize", |b| {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let value = Value::Sequence(SequenceData::Buffer(BuffData { data: data.clone() }));
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    buffer_group.finish();

    // ================================================================================
    // `response` serialization
    // ================================================================================
    let mut response_group = c.benchmark_group("response");

    response_group.bench_function("response-serialize", |b| {
        let value = Value::Response(ResponseData {
            committed: true,
            data: Box::new(Value::Int(5)),
        });

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    response_group.bench_function("response-deserialize", |b| {
        let value = Value::Response(ResponseData {
            committed: true,
            data: Box::new(Value::Int(5)),
        });
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    response_group.finish();

    // ================================================================================
    // `optional` serialization
    // ================================================================================
    let mut optional_group = c.benchmark_group("optional");

    optional_group.bench_function("optional-serialize", |b| {
        let value = Value::Optional(OptionalData {
            data: Some(Box::new(Value::Int(5))),
        });

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });

    optional_group.bench_function("optional-deserialize", |b| {
        let value = Value::Optional(OptionalData {
            data: Some(Box::new(Value::Int(5))),
        });
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    optional_group.finish();

    // ================================================================================
    // `list` serialization
    // ================================================================================
    let mut list_group = c.benchmark_group("list");

    // Empty list.
    list_group.bench_function("list-empty-serialize", |b| {
        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 5)
                .expect("Could not construct list"),
        }));

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });
    list_group.bench_function("list-empty-deserialize", |b| {
        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 5)
                .expect("Could not construct list"),
        }));
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    // List with five signed integer values.
    list_group.bench_function("list-5-ints-serialize", |b| {
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

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });
    list_group.bench_function("list-5-ints-deserialize", |b| {
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

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    // List with five unsigned integer values.
    list_group.bench_function("list-5-uints-serialize", |b| {
        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![
                Value::UInt(1),
                Value::UInt(2),
                Value::UInt(3),
                Value::UInt(4),
                Value::UInt(5),
            ],
            type_signature: ListTypeData::new_list(TypeSignature::UIntType, 5)
                .expect("Could not construct list"),
        }));

        b.iter(|| {
            serialize_clarity_value(&value).expect("Failed to serialize value");
        })
    });
    list_group.bench_function("list-5-uints-deserialize", |b| {
        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![
                Value::UInt(1),
                Value::UInt(2),
                Value::UInt(3),
                Value::UInt(4),
                Value::UInt(5),
            ],
            type_signature: ListTypeData::new_list(TypeSignature::UIntType, 5)
                .expect("Could not construct list"),
        }));
        let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");

        b.iter(|| {
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    list_group.finish();
}
