use hash::output::jsonl::write_json_line;
use hash::pipeline::parallel::OrderedResults;
use serde_json::json;

#[test]
fn ordered_results_flushes_only_when_contiguous() {
    let mut ordered = OrderedResults::new();

    assert!(ordered.push(2, "third").is_empty());
    assert!(ordered.push(4, "fifth").is_empty());
    assert!(ordered.push(1, "second").is_empty());
    assert_eq!(ordered.push(0, "first"), vec!["first", "second", "third"]);
    assert!(ordered.push(6, "seventh").is_empty());
    assert_eq!(ordered.push(3, "fourth"), vec!["fourth", "fifth"]);
    assert_eq!(ordered.push(5, "sixth"), vec!["sixth", "seventh"]);
}

#[test]
fn ordered_results_can_emit_one_output_per_input_in_sequence() {
    let mut ordered = OrderedResults::new();
    let inputs = vec![(3, "d"), (1, "b"), (0, "a"), (2, "c")];

    let mut emitted = Vec::new();
    for (index, value) in inputs {
        emitted.extend(ordered.push(index, value));
    }

    assert_eq!(emitted, vec!["a", "b", "c", "d"]);
    assert_eq!(emitted.len(), 4);
}

#[test]
fn jsonl_writer_emits_compact_single_line_records() {
    let mut output = Vec::new();
    write_json_line(&mut output, &json!({"a":1,"b":"x","nested":{"k":true}}))
        .expect("jsonl write should succeed");

    let rendered = String::from_utf8(output).expect("valid utf8");
    assert_eq!(rendered, "{\"a\":1,\"b\":\"x\",\"nested\":{\"k\":true}}\n");
    assert_eq!(rendered.lines().count(), 1);
}
