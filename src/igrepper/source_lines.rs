use std::cmp;
use std::sync::Arc;

/// Abstraction over the input lines.
/// `Raw` holds the original input lines directly.
/// `Buffered` is a growable buffer that Core populates from a parent OutputGenerator
#[derive(Debug)]
pub(crate) enum SourceLines {
    Raw(Arc<Vec<String>>),
    Buffered {
        buffer: Vec<String>,
        parent_exhausted: bool,
    },
}

impl SourceLines {
    pub fn new_buffered() -> Self {
        SourceLines::Buffered {
            buffer: Vec::new(),
            parent_exhausted: false,
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<String> {
        match self {
            SourceLines::Raw(v) => v.get(index).cloned(),
            SourceLines::Buffered { buffer, .. } => buffer.get(index).cloned(),
        }
    }

    pub(crate) fn is_exhausted(&self, lines_processed: u32) -> bool {
        match self {
            SourceLines::Raw(v) => lines_processed >= v.len() as u32,
            SourceLines::Buffered {
                buffer,
                parent_exhausted,
            } => *parent_exhausted && lines_processed >= buffer.len() as u32,
        }
    }

    pub(crate) fn available_count(&self, lines_processed: u32) -> u32 {
        match self {
            SourceLines::Raw(v) => v.len() as u32,
            SourceLines::Buffered { buffer, .. } => cmp::max(buffer.len() as u32, lines_processed),
        }
    }

    /// Appends lines to a Buffered source and marks whether the parent is done.
    pub(crate) fn extend_buffer(&mut self, new_lines: &[String], parent_exhausted: bool) {
        if let SourceLines::Buffered {
            buffer,
            parent_exhausted: exhausted,
        } = self
        {
            buffer.extend_from_slice(new_lines);
            *exhausted = parent_exhausted;
        }
    }

    pub(crate) fn buffered_len(&self) -> usize {
        match self {
            SourceLines::Raw(v) => v.len(),
            SourceLines::Buffered { buffer, .. } => buffer.len(),
        }
    }
}
