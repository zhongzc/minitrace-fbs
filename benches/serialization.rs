use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn memory_copy(
    minitrace::TraceDetails {
        start_time_ns,
        elapsed_ns,
        cycles_per_second,
        spans,
        properties:
            minitrace::Properties {
                span_ids,
                property_lens,
                payload,
            },
    }: minitrace::TraceDetails,
    buf: &mut Vec<u8>,
) {
    let total_len = std::mem::size_of::<minitrace::TraceDetails>()
        + std::mem::size_of::<minitrace::Span>() * spans.len()
        + std::mem::size_of::<minitrace::Properties>()
        + std::mem::size_of::<u64>() * span_ids.len()
        + std::mem::size_of::<u64>() * property_lens.len()
        + std::mem::size_of::<u8>() * payload.len();
    buf.reserve(total_len);

    buf.extend_from_slice(&start_time_ns.to_be_bytes());
    buf.extend_from_slice(&elapsed_ns.to_be_bytes());
    buf.extend_from_slice(&cycles_per_second.to_be_bytes());

    unsafe {
        let origin = buf.as_mut_ptr();
        let mut cur_ptr = origin.add(buf.len());

        let c = spans.len() * std::mem::size_of::<minitrace::Span>();
        std::ptr::copy_nonoverlapping(spans.as_ptr() as *const u8, cur_ptr, c);
        cur_ptr = cur_ptr.add(c);

        let c = span_ids.len() * std::mem::size_of::<u64>();
        std::ptr::copy_nonoverlapping(span_ids.as_ptr() as *const u8, cur_ptr, c);
        cur_ptr = cur_ptr.add(c);

        let c = property_lens.len() * std::mem::size_of::<u64>();
        std::ptr::copy_nonoverlapping(property_lens.as_ptr() as *const u8, cur_ptr, c);
        cur_ptr = cur_ptr.add(c);

        let c = payload.len() * std::mem::size_of::<u8>();
        std::ptr::copy_nonoverlapping(payload.as_ptr() as *const u8, cur_ptr, c);
        cur_ptr = cur_ptr.add(c);

        buf.set_len(cur_ptr as usize - origin as usize);
    }
}

#[derive(Debug, Copy, Clone)]
enum Type {
    Memcpy,
    Flatbuffers,
}

fn serizalization(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "serialization",
        |b, tp| {
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
                            }
                        });

                        jh.join().unwrap();
                        collector
                    };

                    collector.collect()
                },
                |trace_result| match tp {
                    Type::Memcpy => {
                        let mut buf = Vec::with_capacity(8192);
                        memory_copy(trace_result, &mut buf);
                        black_box(buf);
                    }
                    Type::Flatbuffers => {
                        let mut builder = flatbuffers::FlatBufferBuilder::new_with_capacity(8192);
                        black_box(
                            minitrace_fbs::serialize_to_fbs(&mut builder, trace_result).unwrap(),
                        );
                    }
                },
            );
        },
        vec![Type::Memcpy, Type::Flatbuffers],
    );
}

criterion_group!(benches, serizalization);
criterion_main!(benches);
