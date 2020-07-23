use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn serizalization(c: &mut Criterion) {
    c.bench_function("flatbuffers", |b| {
        b.iter_with_setup(
            || {
                let collector = {
                    let (_root, collector) = minitrace::trace_enable(0u32);

                    for i in 1..50u32 {
                        let _guard = minitrace::new_span(i);
                    }

                    let handle = minitrace::trace_crossthread();

                    let jh = std::thread::spawn(move || {
                        let mut handle = handle;
                        let _guard = handle.trace_enable(50u32);
                        for i in 51..100u32 {
                            let _guard = minitrace::new_span(i);
                            minitrace::property(b"wuhu~fly~");
                        }
                    });

                    jh.join().unwrap();
                    collector
                };

                collector.collect()
            },
            |trace_result| {
                let mut builder = flatbuffers::FlatBufferBuilder::new_with_capacity(8192);
                black_box(minitrace_fbs::serialize_to_fbs_p(&mut builder, &trace_result).unwrap());
            },
        );
    });
}

criterion_group!(benches, serizalization);
criterion_main!(benches);
