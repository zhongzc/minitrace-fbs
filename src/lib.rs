use std::error::Error;

#[allow(unused_imports, dead_code)]
mod minitrace_generated;
use minitrace_generated::*;

pub fn serialize_to_fbs<'a>(
    builder: &'a mut flatbuffers::FlatBufferBuilder,
    origin: &minitrace::TraceDetails,
) -> Result<&'a [u8], Box<dyn Error>> {
    // allocation
    let mut span_sets = Vec::with_capacity(origin.span_sets.len());
    let mut spans_buf = Vec::with_capacity(1024);

    for span_set in &origin.span_sets {
        // encode spans
        spans_buf.clear();
        for span in &span_set.spans {
            let (link_type, link_id) = match span.link {
                minitrace::Link::Root => (LinkType::Root, 0),
                minitrace::Link::Parent { id } => (LinkType::Parent, id),
                minitrace::Link::Continue { id } => (LinkType::Continue, id),
            };
            let span = Span::new(
                span.id,
                link_type,
                link_id,
                span.begin_cycles,
                span.elapsed_cycles,
                span.event,
            );
            spans_buf.push(span);
        }
        let spans = builder.create_vector_direct(&spans_buf);

        // encode properties
        let span_ids = builder.create_vector_direct(&span_set.properties.span_ids);
        let lens = builder.create_vector_direct(&span_set.properties.span_lens);
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
