use std::collections::HashMap;
use std::error::Error;

#[allow(unused_imports, dead_code)]
mod minitrace_generated;
use minitrace_generated::*;

fn build_id_mapper(origin: &minitrace::TraceDetails) -> Option<HashMap<u64, u16>> {
    let total_len = origin.span_sets.iter().map(|ss| ss.spans.len()).sum();
    if total_len >= std::u16::MAX as usize {
        return None;
    }
    let mut id = 0u16;
    let mut mapper = HashMap::with_capacity(total_len);

    for span_set in &origin.span_sets {
        for span in &span_set.spans {
            id += 1;
            assert_eq!(mapper.insert(span.id, id), None);
        }
    }

    Some(mapper)
}

pub fn serialize_to_fbs<'a>(
    builder: &'a mut flatbuffers::FlatBufferBuilder,
    origin: &minitrace::TraceDetails,
) -> Result<&'a [u8], Box<dyn Error>> {
    let mapper = build_id_mapper(origin).ok_or("too much spans: > 65535")?;

    // allocation
    let mut span_sets = Vec::with_capacity(origin.span_sets.len());
    let mut span_ids_buf = Vec::with_capacity(1024);
    let mut lens_buf = Vec::with_capacity(1024);
    let mut spans_buf = Vec::with_capacity(1024);

    for span_set in &origin.span_sets {
        // encode spans
        spans_buf.clear();
        for span in &span_set.spans {
            let id = mapper[&span.id];
            let (link_type, link_id) = match span.link {
                minitrace::Link::Root => (LinkType::Root, 0),
                minitrace::Link::Parent { id } => (LinkType::Parent, mapper[&id]),
                minitrace::Link::Continue { id } => (LinkType::Continue, mapper[&id]),
            };
            let span = Span::create(
                builder,
                &SpanArgs {
                    id,
                    link_type,
                    link_id,
                    begin_cycles: span.begin_cycles,
                    elapsed_cycles: span.elapsed_cycles,
                    event: span.event,
                },
            );
            spans_buf.push(span);
        }
        let spans = builder.create_vector(&spans_buf);

        // encode properties
        span_ids_buf.clear();
        lens_buf.clear();
        for (id, len) in &span_set.properties.span_id_to_len {
            if *len > std::u16::MAX as u64 {
                return Err("property lenght > 65536".into());
            }
            span_ids_buf.push(mapper[id]);
            lens_buf.push(*len as u16);
        }
        let span_ids = builder.create_vector_direct(&span_ids_buf);
        let lens = builder.create_vector_direct(&lens_buf);
        let payload = builder.create_vector_direct(&span_set.properties.payload);
        let properties = Properties::create(
            builder,
            &PropertiesArgs {
                span_ids: Some(span_ids),
                lens: Some(lens),
                payload: Some(payload),
            },
        );

        // encode span_set
        let span_set_ = SpanSet::create(
            builder,
            &SpanSetArgs {
                create_time_ns: span_set.create_time_ns,
                start_time_ns: span_set.start_time_ns,
                spans: Some(spans),
                properties: Some(properties),
            },
        );

        span_sets.push(span_set_);
    }

    let span_sets = builder.create_vector(&span_sets);

    let trace_details = TraceDetails::create(
        builder,
        &TraceDetailsArgs {
            start_time_ns: origin.start_time_ns,
            elapsed_ns: origin.elapsed_ns,
            cycles_per_second: origin.cycles_per_second,
            span_sets: Some(span_sets),
        },
    );

    builder.finish(trace_details, None);
    Ok(builder.finished_data())
}

pub fn serialize_to_fbs_p<'a>(
    builder: &'a mut flatbuffers::FlatBufferBuilder,
    origin: &minitrace::TraceDetails,
) -> Result<&'a [u8], Box<dyn Error>> {
    // allocation
    let mut span_sets = Vec::with_capacity(origin.span_sets.len());
    let mut span_ids_buf = Vec::with_capacity(1024);
    let mut lens_buf = Vec::with_capacity(1024);
    let mut spans_buf = Vec::with_capacity(1024);

    for span_set in &origin.span_sets {
        // encode spans
        spans_buf.clear();
        for span in &span_set.spans {
            let id = span.id as u16;
            let (link_type, link_id) = match span.link {
                minitrace::Link::Root => (LinkType::Root, 0),
                minitrace::Link::Parent { id } => (LinkType::Parent, id as u16),
                minitrace::Link::Continue { id } => (LinkType::Continue, id as u16),
            };
            let span = Span::create(
                builder,
                &SpanArgs {
                    id,
                    link_type,
                    link_id,
                    begin_cycles: span.begin_cycles,
                    elapsed_cycles: span.elapsed_cycles,
                    event: span.event,
                },
            );
            spans_buf.push(span);
        }
        let spans = builder.create_vector(&spans_buf);

        // encode properties
        span_ids_buf.clear();
        lens_buf.clear();
        for (id, len) in &span_set.properties.span_id_to_len {
            if *len > std::u16::MAX as u64 {
                return Err("property lenght > 65536".into());
            }
            span_ids_buf.push(*id as u16);
            lens_buf.push(*len as u16);
        }
        let span_ids = builder.create_vector_direct(&span_ids_buf);
        let lens = builder.create_vector_direct(&lens_buf);
        let payload = builder.create_vector_direct(&span_set.properties.payload);
        let properties = Properties::create(
            builder,
            &PropertiesArgs {
                span_ids: Some(span_ids),
                lens: Some(lens),
                payload: Some(payload),
            },
        );

        // encode span_set
        let span_set_ = SpanSet::create(
            builder,
            &SpanSetArgs {
                create_time_ns: span_set.create_time_ns,
                start_time_ns: span_set.start_time_ns,
                spans: Some(spans),
                properties: Some(properties),
            },
        );

        span_sets.push(span_set_);
    }

    let span_sets = builder.create_vector(&span_sets);

    let trace_details = TraceDetails::create(
        builder,
        &TraceDetailsArgs {
            start_time_ns: origin.start_time_ns,
            elapsed_ns: origin.elapsed_ns,
            cycles_per_second: origin.cycles_per_second,
            span_sets: Some(span_sets),
        },
    );

    builder.finish(trace_details, None);
    Ok(builder.finished_data())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
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

        let trace_result = collector.collect();
        dbg!(&trace_result);

        let mut builder = flatbuffers::FlatBufferBuilder::new_with_capacity(8192);
        let data = serialize_to_fbs(&mut builder, &trace_result).unwrap();
        dbg!(data.len());
    }
}
