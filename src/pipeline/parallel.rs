use rayon::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct OrderedResults<T> {
    next_index: usize,
    pending: BTreeMap<usize, T>,
}

impl<T> Default for OrderedResults<T> {
    fn default() -> Self {
        Self {
            next_index: 0,
            pending: BTreeMap::new(),
        }
    }
}

impl<T> OrderedResults<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, index: usize, value: T) -> Vec<T> {
        self.pending.insert(index, value);

        let mut ready = Vec::new();
        while let Some(next_value) = self.pending.remove(&self.next_index) {
            ready.push(next_value);
            self.next_index += 1;
        }

        ready
    }
}

pub fn normalized_jobs(requested: Option<usize>) -> usize {
    match requested {
        Some(0) => 1,
        Some(jobs) => jobs,
        None => std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1),
    }
}

pub fn process_indexed_in_parallel<T, U, F>(items: Vec<T>, jobs: usize, process: F) -> Vec<U>
where
    T: Send,
    U: Send,
    F: Fn((usize, T)) -> U + Sync + Send,
{
    let worker_count = jobs.max(1);
    if worker_count == 1 {
        return items.into_iter().enumerate().map(&process).collect();
    }

    match rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
    {
        Ok(pool) => {
            pool.install(|| {
                // Vec::into_par_iter() is indexed; collect() preserves original order.
                items.into_par_iter().enumerate().map(&process).collect()
            })
        }
        Err(_) => {
            // Fallback to sequential processing if thread pool creation fails
            items.into_iter().enumerate().map(&process).collect()
        }
    }
}
