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
