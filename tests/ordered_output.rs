use hash::output::jsonl::write_json_line;
use hash::pipeline::parallel::{OrderedResults, normalized_jobs, process_indexed_in_parallel};
use serde_json::json;
use std::time::Duration;

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

#[test]
fn jobs_normalization_honors_defaults_and_lower_bound() {
    assert!(normalized_jobs(None) >= 1);
    assert_eq!(normalized_jobs(Some(0)), 1);
    assert_eq!(normalized_jobs(Some(6)), 6);
}

#[test]
fn parallel_processing_matches_sequential_order_and_values() {
    fn simulate_work((index, value): (usize, usize)) -> String {
        let delay_ms = ((17 - (index % 17)) % 17) as u64;
        std::thread::sleep(Duration::from_millis(delay_ms));
        format!("{index}:{value}")
    }

    let inputs: Vec<usize> = (0..64).collect();

    let sequential = process_indexed_in_parallel(inputs.clone(), 1, simulate_work);
    let parallel = process_indexed_in_parallel(inputs, 4, simulate_work);

    assert_eq!(parallel, sequential);
}
