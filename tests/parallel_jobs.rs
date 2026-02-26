use hash::pipeline::parallel::{normalized_jobs, process_indexed_in_parallel};
use std::time::Duration;

#[test]
fn normalized_jobs_defaults_to_available_parallelism_or_one() {
    assert!(normalized_jobs(None) >= 1);
}

#[test]
fn normalized_jobs_clamps_zero_to_one() {
    assert_eq!(normalized_jobs(Some(0)), 1);
    assert_eq!(normalized_jobs(Some(1)), 1);
    assert_eq!(normalized_jobs(Some(8)), 8);
}

#[test]
fn process_indexed_in_parallel_is_deterministic_for_jobs_one() {
    let inputs = vec!["a", "b", "c", "d"];
    let output =
        process_indexed_in_parallel(inputs, 1, |(index, value)| format!("{index}:{value}"));

    assert_eq!(output, vec!["0:a", "1:b", "2:c", "3:d"]);
}

#[test]
fn process_indexed_in_parallel_is_deterministic_for_jobs_many() {
    let inputs = vec![5_u64, 1, 4, 0, 3, 2];
    let output = process_indexed_in_parallel(inputs, 4, |(index, delay_ms)| {
        std::thread::sleep(Duration::from_millis(delay_ms));
        index
    });

    assert_eq!(output, vec![0, 1, 2, 3, 4, 5]);
}
