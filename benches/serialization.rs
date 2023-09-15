use clarity::vm::{
    types::{
        ASCIIData, BuffData, CharType, ListData, ListTypeData, OptionalData, ResponseData,
        SequenceData, TypeSignature, UTF8Data,
    },
    Value,
};
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_test::serialization::*;

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("uint", |b| {
        let value = Value::UInt(u128::MAX);

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    c.bench_function("int", |b| {
        let value = Value::Int(i128::MAX);

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    c.bench_function("bool", |b| {
        let value = Value::Bool(true);

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    c.bench_function("string-ascii", |b| {
        let value = Value::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
            data: "hello world!".as_bytes().to_vec(),
        })));

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    c.bench_function("string-utf8", |b| {
        let str = "hello world!";
        let mut data = Vec::<Vec<u8>>::new();
        for chunk in str.as_bytes().chunks(4) {
            data.push(vec![chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        let value = Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data { data })));

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    c.bench_function("buffer", |b| {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let value = Value::Sequence(SequenceData::Buffer(BuffData { data: data.clone() }));

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    c.bench_function("response", |b| {
        let value = Value::Response(ResponseData {
            committed: true,
            data: Box::new(Value::Int(5)),
        });

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    c.bench_function("optional", |b| {
        let value = Value::Optional(OptionalData {
            data: Some(Box::new(Value::Int(5))),
        });

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    let mut group = c.benchmark_group("Lists");

    group.bench_function("list-empty", |b| {
        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![
            ],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 5)
                .expect("Could not construct list"),
        }));

        b.iter(|| {
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    group.bench_function("list-5-ints", |b| {
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
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    group.bench_function("list-5-uints", |b| {
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
            let serialized = serialize_clarity_value(&value).expect("Failed to serialize value");
            deserialize_clarity_value(&serialized).expect("Failed to deserialize value");
        })
    });

    group.finish();
}
