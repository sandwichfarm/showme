use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// Holds base64-encoded data together with metadata about how to chunk it.
pub(crate) struct Base64Chunks {
    encoded: String,
    chunk_size: usize,
    chunk_count: usize,
}

impl Base64Chunks {
    pub(crate) fn new(data: &[u8], chunk_size: usize) -> Self {
        let chunk_size = sanitize_chunk_size(chunk_size);
        let encoded = STANDARD.encode(data);
        let chunk_count = if encoded.is_empty() {
            1
        } else {
            (encoded.len() + chunk_size - 1) / chunk_size
        };
        Self {
            encoded,
            chunk_size,
            chunk_count,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.chunk_count
    }

    pub(crate) fn iter(&self) -> Base64ChunkIter<'_> {
        Base64ChunkIter {
            encoded: &self.encoded,
            chunk_size: self.chunk_size,
            offset: 0,
            yielded_empty: false,
        }
    }

    pub(crate) fn encoded_len(&self) -> usize {
        self.encoded.len()
    }
}

impl<'a> IntoIterator for &'a Base64Chunks {
    type Item = &'a str;
    type IntoIter = Base64ChunkIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub(crate) struct Base64ChunkIter<'a> {
    encoded: &'a str,
    chunk_size: usize,
    offset: usize,
    yielded_empty: bool,
}

impl<'a> Iterator for Base64ChunkIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.encoded.is_empty() {
            if self.yielded_empty {
                return None;
            }
            self.yielded_empty = true;
            return Some("");
        }

        if self.offset >= self.encoded.len() {
            return None;
        }

        let end = usize::min(self.offset + self.chunk_size, self.encoded.len());
        let chunk = &self.encoded[self.offset..end];
        self.offset = end;
        Some(chunk)
    }
}

fn sanitize_chunk_size(chunk_size: usize) -> usize {
    let chunk_size = chunk_size.max(4);
    let remainder = chunk_size % 4;
    if remainder == 0 {
        chunk_size
    } else {
        chunk_size + (4 - remainder)
    }
}

pub(crate) fn average_chunk_len(chunks: &Base64Chunks) -> usize {
    if chunks.len() == 0 {
        return 0;
    }
    let len = chunks.encoded_len();
    if len == 0 {
        0
    } else {
        (len + chunks.len() - 1) / chunks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_single_empty_chunk() {
        let chunks = Base64Chunks::new(&[], 1024);
        let collected: Vec<&str> = (&chunks).into_iter().collect();
        assert_eq!(chunks.len(), 1);
        assert_eq!(collected, vec![""]);
    }

    #[test]
    fn chunking_respects_requested_size() {
        let data = vec![0u8; 8192];
        let chunks = Base64Chunks::new(&data, 1024);
        let pieces: Vec<&str> = (&chunks).into_iter().collect();
        assert!(pieces.len() > 1);
        for piece in &pieces[..pieces.len() - 1] {
            assert_eq!(piece.len() % 4, 0);
            assert!(piece.len() <= sanitize_chunk_size(1024));
        }
    }

    #[test]
    fn sanitize_rounds_up_to_multiple_of_four() {
        assert_eq!(sanitize_chunk_size(1), 4);
        assert_eq!(sanitize_chunk_size(6), 8);
        assert_eq!(sanitize_chunk_size(8), 8);
    }
}
