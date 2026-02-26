use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// An ordered writer that buffers results and writes them in sequence order
/// regardless of the order in which they are received from parallel workers.
pub struct OrderedWriter<W> {
    writer: W,
    buffer: BTreeMap<u64, String>,
    next_expected: u64,
}

impl<W: Write> OrderedWriter<W> {
    /// Create a new ordered writer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            buffer: BTreeMap::new(),
            next_expected: 0,
        }
    }

    /// Write a value with its sequence number. Values will be buffered
    /// and written in sequence order regardless of submission order.
    pub fn write_ordered<T>(&mut self, sequence: u64, value: &T) -> io::Result<()>
    where
        T: Serialize,
    {
        // Serialize to JSON line format
        let mut json_line = serde_json::to_string(value).map_err(io::Error::other)?;
        json_line.push('\n');

        // If this is the next expected sequence, write immediately and flush buffer
        if sequence == self.next_expected {
            self.writer.write_all(json_line.as_bytes())?;
            self.next_expected += 1;

            // Write any consecutive buffered items
            while let Some(buffered_line) = self.buffer.remove(&self.next_expected) {
                self.writer.write_all(buffered_line.as_bytes())?;
                self.next_expected += 1;
            }
        } else {
            // Buffer for later
            self.buffer.insert(sequence, json_line);
        }

        Ok(())
    }

    /// Flush any remaining writes to the underlying writer
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Get the underlying writer back, ensuring all buffered items are written
    pub fn into_inner(mut self) -> io::Result<W> {
        // Write any remaining buffered items in order
        let buffered_items: Vec<_> = self.buffer.into_iter().collect();
        for (_, buffered_line) in buffered_items {
            self.writer.write_all(buffered_line.as_bytes())?;
        }
        self.writer.flush()?;
        Ok(self.writer)
    }

    /// Check if there are any buffered items waiting to be written
    pub fn has_buffered(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Get the count of buffered items
    pub fn buffered_count(&self) -> usize {
        self.buffer.len()
    }

    /// Get the next expected sequence number
    pub fn next_expected_sequence(&self) -> u64 {
        self.next_expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn writes_in_order_when_received_in_order() {
        let mut output = Vec::new();
        let mut writer = OrderedWriter::new(&mut output);

        writer.write_ordered(0, &json!({"id": 0})).unwrap();
        writer.write_ordered(1, &json!({"id": 1})).unwrap();
        writer.write_ordered(2, &json!({"id": 2})).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, "{\"id\":0}\n{\"id\":1}\n{\"id\":2}\n");
    }

    #[test]
    fn buffers_and_reorders_out_of_order_writes() {
        let mut output = Vec::new();
        let mut writer = OrderedWriter::new(&mut output);

        // Write out of order: 0, 2, 1
        writer.write_ordered(0, &json!({"id": 0})).unwrap();
        writer.write_ordered(2, &json!({"id": 2})).unwrap();
        writer.write_ordered(1, &json!({"id": 1})).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, "{\"id\":0}\n{\"id\":1}\n{\"id\":2}\n");
    }

    #[test]
    fn handles_large_gaps_in_sequence() {
        let mut output = Vec::new();
        let mut writer = OrderedWriter::new(&mut output);

        // Write with a large gap: 0, 5, 3, 1, 4, 2
        writer.write_ordered(0, &json!({"id": 0})).unwrap();
        writer.write_ordered(5, &json!({"id": 5})).unwrap();
        writer.write_ordered(3, &json!({"id": 3})).unwrap();
        writer.write_ordered(1, &json!({"id": 1})).unwrap();
        writer.write_ordered(4, &json!({"id": 4})).unwrap();
        writer.write_ordered(2, &json!({"id": 2})).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(
            result,
            "{\"id\":0}\n{\"id\":1}\n{\"id\":2}\n{\"id\":3}\n{\"id\":4}\n{\"id\":5}\n"
        );
    }

    #[test]
    fn tracks_buffered_count_correctly() {
        let mut output = Vec::new();
        let mut writer = OrderedWriter::new(&mut output);

        assert_eq!(writer.buffered_count(), 0);
        assert!(!writer.has_buffered());

        // Write 0 - should write immediately
        writer.write_ordered(0, &json!({"id": 0})).unwrap();
        assert_eq!(writer.buffered_count(), 0);

        // Write 2 - should buffer
        writer.write_ordered(2, &json!({"id": 2})).unwrap();
        assert_eq!(writer.buffered_count(), 1);
        assert!(writer.has_buffered());

        // Write 1 - should flush both 1 and 2
        writer.write_ordered(1, &json!({"id": 1})).unwrap();
        assert_eq!(writer.buffered_count(), 0);
        assert!(!writer.has_buffered());
    }

    #[test]
    fn into_inner_flushes_buffered_items() {
        let mut output = Vec::new();
        {
            let mut writer = OrderedWriter::new(&mut output);

            // Write some out of order items that won't be flushed
            writer.write_ordered(1, &json!({"id": 1})).unwrap();
            writer.write_ordered(2, &json!({"id": 2})).unwrap();

            // into_inner should write buffered items even without sequence 0
            writer.into_inner().unwrap();
        }

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, "{\"id\":1}\n{\"id\":2}\n");
    }

    #[test]
    fn tracks_next_expected_sequence() {
        let mut output = Vec::new();
        let mut writer = OrderedWriter::new(&mut output);

        assert_eq!(writer.next_expected_sequence(), 0);

        writer.write_ordered(0, &json!({"id": 0})).unwrap();
        assert_eq!(writer.next_expected_sequence(), 1);

        writer.write_ordered(1, &json!({"id": 1})).unwrap();
        assert_eq!(writer.next_expected_sequence(), 2);

        // Write out of order - next expected shouldn't change until gap is filled
        writer.write_ordered(3, &json!({"id": 3})).unwrap();
        assert_eq!(writer.next_expected_sequence(), 2);

        writer.write_ordered(2, &json!({"id": 2})).unwrap();
        assert_eq!(writer.next_expected_sequence(), 4);
    }
}
