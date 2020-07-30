use std::error::Error;

#[allow(unused_imports, dead_code)]
mod minitrace_generated;
use minitrace_generated::*;

pub fn serialize_to_fbs<'a>(
    builder: &'a mut flatbuffers::FlatBufferBuilder,
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
) -> Result<&'a [u8], Box<dyn Error>> {
    let mut spans_buf = Vec::with_capacity(spans.len());

    for minitrace::Span {
        id,
        state,
        related_id,
        begin_cycles,
        elapsed_cycles,
        event,
    } in spans
    {
        spans_buf.push(Span::new(
            id,
            match state {
                minitrace::State::Root => State::Root,
                minitrace::State::Local => State::Local,
                minitrace::State::Spawning => State::Spawning,
                minitrace::State::Scheduling => State::Scheduling,
                minitrace::State::Settle => State::Settle,
            },
            related_id,
            begin_cycles,
            elapsed_cycles,
            event,
        ))
    }

    let span_ids = Some(builder.create_vector_direct(&span_ids));
    let property_lens = Some(builder.create_vector_direct(&property_lens));
    let payload = Some(builder.create_vector_direct(&payload));
    let properties = Properties::create(
        builder,
        &PropertiesArgs {
            span_ids,
            property_lens,
            payload,
        },
    );

    let spans = builder.create_vector_direct(&spans_buf);

    let trace_details = TraceDetails::create(
        builder,
        &TraceDetailsArgs {
            start_time_ns,
            elapsed_ns,
            cycles_per_second,
            spans: Some(spans),
            properties: Some(properties),
        },
    );

    builder.finish(trace_details, None);
    Ok(builder.finished_data())
}
