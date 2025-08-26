use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use wind_core::{WindValue, MessageCodec, Message, MessagePayload};
use std::collections::HashMap;

fn bench_message_serialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("message_serialization");
    
    for payload_size in [64, 256, 1024, 4096].iter() {
        let mut payload = HashMap::new();
        payload.insert("data".to_string(), WindValue::Bytes(vec![0u8; *payload_size]));
        payload.insert("timestamp".to_string(), WindValue::I64(12345678));
        payload.insert("sequence".to_string(), WindValue::I64(1));
        
        let message = Message::new(MessagePayload::Publish {
            service: "TEST/BENCH".to_string(),
            sequence: 1,
            value: WindValue::Map(payload),
            schema_id: None,
        });
        
        group.bench_with_input(
            BenchmarkId::new("encode", payload_size),
            payload_size,
            |b, _| {
                b.iter(|| {
                    let encoded = MessageCodec::encode(&message).unwrap();
                    black_box(encoded);
                });
            }
        );
        
        let encoded = MessageCodec::encode(&message).unwrap();
        group.bench_with_input(
            BenchmarkId::new("decode", payload_size),
            payload_size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let mut cursor = std::io::Cursor::new(&encoded[4..]); // Skip length prefix
                    let decoded: Message = bincode::deserialize_from(cursor).unwrap();
                    black_box(decoded);
                });
            }
        );
    }
    
    group.finish();
}

fn bench_wind_value_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("wind_value_conversion");
    
    // Bench primitive conversions
    group.bench_function("bool_conversion", |b| {
        b.iter(|| {
            let val: WindValue = black_box(true).into();
            let converted: bool = val.try_into().unwrap();
            black_box(converted);
        });
    });
    
    group.bench_function("i32_conversion", |b| {
        b.iter(|| {
            let val: WindValue = black_box(42i32).into();
            let converted: i32 = val.try_into().unwrap();
            black_box(converted);
        });
    });
    
    group.bench_function("string_conversion", |b| {
        let test_str = "Hello, WIND!".to_string();
        b.iter(|| {
            let val: WindValue = black_box(test_str.clone()).into();
            let converted: String = val.try_into().unwrap();
            black_box(converted);
        });
    });
    
    // Bench complex type conversions
    group.bench_function("map_creation", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            map.insert("field1".to_string(), WindValue::I32(black_box(42)));
            map.insert("field2".to_string(), WindValue::String(black_box("test".to_string())));
            map.insert("field3".to_string(), WindValue::F64(black_box(3.14)));
            let val = WindValue::Map(map);
            black_box(val);
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_message_serialization, bench_wind_value_conversion);
criterion_main!(benches);
