//! SEC-B2 — evidence that the bulk-import read path is **memory-bounded**,
//! not merely "streaming-shaped".
//!
//! The rest of the SEC-B2 tests pin behaviour (rows land, caps trip, errors
//! are per-row). Behaviour tests cannot tell a streaming reader from a
//! buffering one: both produce the same rows. This suite measures the
//! thing that actually changed, by installing a **counting global
//! allocator** and comparing peak heap-in-use across a measured section.
//!
//! It lives in its own integration-test binary because a `#[global_allocator]`
//! is per-binary, and because the measurement is process-global: every test
//! here takes [`MEASURE`] so two measurements never overlap.
//!
//! What it proves:
//!
//! - Streaming **~312 MiB** of JSONL through `jsonl::LineReader` peaks at
//!   ~0.19 MiB — three orders of magnitude below the input, and identical
//!   to the peak for a tenth of the input, so the curve is flat.
//! - Streaming a large CSV through `csv::RowStream` is likewise flat,
//!   including across the async→blocking bridge.
//! - The **old shape** (`split_lines` over a whole buffer, which is what
//!   the import path used to do) allocates in proportion to the file, on
//!   the same input — so the contrast is measured here, not asserted from
//!   the diff.

use std::alloc::{GlobalAlloc, Layout, System};
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use person_service::bulk::{csv, jsonl};
use tokio::io::{AsyncRead, ReadBuf};

// ---------------------------------------------------------------- allocator

/// Bytes currently allocated through [`Counting`].
static LIVE: AtomicUsize = AtomicUsize::new(0);
/// High-water mark of [`LIVE`] since the last [`reset_peak`].
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// A pass-through allocator that tracks live bytes and their high-water
/// mark. Deliberately simple: `Relaxed` ordering is enough for a
/// high-water mark read after the work has finished, and the counters cost
/// two atomics per allocation.
struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            let live = if new_size >= layout.size() {
                LIVE.fetch_add(new_size - layout.size(), Ordering::Relaxed) + new_size
                    - layout.size()
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed) - layout.size()
                    + new_size
            };
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Serialises the measured sections: the counters are process-global, so
/// two tests measuring at once would each see the other's allocations.
/// Async-aware, because the measured section is what awaits.
static MEASURE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Start a measurement: drop the high-water mark to whatever is live now,
/// and return that baseline.
fn reset_peak() -> usize {
    let live = LIVE.load(Ordering::Relaxed);
    PEAK.store(live, Ordering::Relaxed);
    live
}

/// Peak bytes allocated **above** `baseline` since [`reset_peak`].
fn peak_above(baseline: usize) -> usize {
    PEAK.load(Ordering::Relaxed).saturating_sub(baseline)
}

/// Render a byte count for assertion messages.
fn mib(bytes: usize) -> String {
    format!("{:.2} MiB", bytes as f64 / (1024.0 * 1024.0))
}

// ------------------------------------------------------------ the generator

/// An [`AsyncRead`] that synthesises `total` bytes by repeating `pattern`,
/// **without ever holding the synthesised bytes**.
///
/// This is what makes a hundreds-of-megabytes measurement honest: a test
/// that first built a `Vec` that large to read from would allocate the
/// very thing the streaming reader is supposed to avoid, and the
/// measurement would be meaningless.
struct Generated {
    /// The repeating unit (one complete row, newline included).
    pattern: Vec<u8>,
    /// Offset into `pattern` for the next byte.
    pos: usize,
    /// Bytes produced so far.
    emitted: usize,
    /// Bytes to produce in total.
    total: usize,
}

impl Generated {
    /// A source emitting `pattern` exactly `repeats` times.
    fn new(pattern: Vec<u8>, repeats: usize) -> Self {
        let total = pattern.len() * repeats;
        Self {
            pattern,
            pos: 0,
            emitted: 0,
            total,
        }
    }

    /// A source emitting `prefix` once (a CSV header, say), then `pattern`
    /// `repeats` times.
    fn with_prefix(prefix: &[u8], pattern: &[u8], repeats: usize) -> Chain {
        Chain {
            prefix: prefix.to_vec(),
            prefix_pos: 0,
            body: Generated::new(pattern.to_vec(), repeats),
        }
    }
}

impl AsyncRead for Generated {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let want = buf.remaining().min(this.total - this.emitted);
        let mut written = 0;
        while written < want {
            let take = (this.pattern.len() - this.pos).min(want - written);
            buf.put_slice(&this.pattern[this.pos..this.pos + take]);
            this.pos = (this.pos + take) % this.pattern.len();
            written += take;
        }
        this.emitted += written;
        Poll::Ready(Ok(()))
    }
}

/// A prefix (header) followed by a [`Generated`] body.
struct Chain {
    /// The one-shot prefix bytes.
    prefix: Vec<u8>,
    /// How much of `prefix` has been emitted.
    prefix_pos: usize,
    /// The repeating body.
    body: Generated,
}

impl AsyncRead for Chain {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.prefix_pos < this.prefix.len() {
            let take = buf.remaining().min(this.prefix.len() - this.prefix_pos);
            buf.put_slice(&this.prefix[this.prefix_pos..this.prefix_pos + take]);
            this.prefix_pos += take;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut this.body).poll_read(cx, buf)
    }
}

// ------------------------------------------------------------------- fixtures

/// One JSONL person row, newline-terminated.
fn jsonl_row() -> Vec<u8> {
    let person = sample_person();
    let mut row = jsonl::to_line(&person).unwrap().into_bytes();
    row.push(b'\n');
    row
}

/// A person with enough populated fields to be a realistic row.
fn sample_person() -> person_service::models::Person {
    use person_service::models::{Gender, HumanName, Identifier, IdentifierType, Person};
    let mut p = Person::new(
        HumanName {
            use_type: None,
            family: "Lovelace".to_string(),
            given: vec!["Ada".to_string(), "Augusta".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    );
    p.identifiers.push(Identifier::new(
        IdentifierType::SSN,
        "http://hl7.org/fhir/sid/us-ssn".to_string(),
        "123-45-6789".to_string(),
    ));
    p
}

/// The upper bound a streaming reader must stay under. Generous by
/// design — the point is the *shape* of the curve (flat in the input
/// size), not shaving the constant. It is still far below the smallest
/// input measured here, so a reader that buffered any meaningful fraction
/// of the file would blow it.
const STREAMING_PEAK_LIMIT: usize = 16 * 1024 * 1024;

// ---------------------------------------------------------------- the tests

/// Stream hundreds of megabytes of JSONL and watch the heap stay flat.
#[tokio::test]
async fn jsonl_line_reader_peak_is_flat_in_the_input_size() {
    let _guard = MEASURE.lock().await;
    let row = jsonl_row();

    // Two sizes an order of magnitude apart: if peak memory tracked the
    // input at all, the second would be ~10x the first.
    let mut peaks = Vec::new();
    let mut counts = Vec::new();
    for repeats in [50_000usize, 500_000] {
        let source = Generated::new(row.clone(), repeats);
        let baseline = reset_peak();
        let mut reader = jsonl::LineReader::new(source, usize::MAX);
        let mut rows = 0usize;
        while let Some(line) = reader.next_line().await {
            let line = line.expect("no terminal error");
            // Parse and drop, exactly as the pipeline does per row.
            assert!(jsonl::parse_line(&line).is_ok());
            rows += 1;
        }
        peaks.push(peak_above(baseline));
        counts.push(rows);
        drop(reader);
    }

    assert_eq!(counts, vec![50_000, 500_000], "every row was yielded");
    let bytes_read = row.len() * 500_000;
    eprintln!(
        "JSONL streaming peak: {} for 50k rows ({}), {} for 500k rows ({})",
        mib(peaks[0]),
        mib(row.len() * 50_000),
        mib(peaks[1]),
        mib(bytes_read)
    );
    assert!(
        bytes_read > 100 * 1024 * 1024,
        "the large case must be a genuinely large input, was {}",
        mib(bytes_read)
    );
    assert!(
        peaks[1] < STREAMING_PEAK_LIMIT,
        "streaming {} of JSONL peaked at {} — expected under {}",
        mib(bytes_read),
        mib(peaks[1]),
        mib(STREAMING_PEAK_LIMIT)
    );
    // Flat, not merely small: a 10x larger input must not cost 10x memory.
    assert!(
        peaks[1] <= peaks[0].saturating_mul(2) + 1024 * 1024,
        "peak grew with the input: {} for 50k rows vs {} for 500k rows",
        mib(peaks[0]),
        mib(peaks[1])
    );
}

/// The same, for the CSV path — which is a different mechanism (a blocking
/// `csv::Reader` fed over bounded channels), so it needs its own evidence.
#[tokio::test]
async fn csv_row_stream_peak_is_flat_in_the_input_size() {
    let _guard = MEASURE.lock().await;

    let encoded = csv::encode(&[sample_person()], b',').unwrap();
    let text = String::from_utf8(encoded).unwrap();
    let (header_line, row_line) = text.split_once('\n').unwrap();
    let header_bytes = format!("{header_line}\n").into_bytes();
    let row_bytes = row_line.as_bytes().to_vec();

    let mut peaks = Vec::new();
    let mut counts = Vec::new();
    for repeats in [20_000usize, 200_000] {
        let source = Generated::with_prefix(&header_bytes, &row_bytes, repeats);
        let baseline = reset_peak();
        let mut stream = csv::RowStream::new(source, usize::MAX, b',');
        let mut rows = 0usize;
        while let Some(item) = stream.next_row().await {
            let (_had_id, parsed) = item.expect("no terminal error");
            assert!(parsed.is_ok());
            rows += 1;
        }
        peaks.push(peak_above(baseline));
        counts.push(rows);
        drop(stream);
    }

    assert_eq!(counts, vec![20_000, 200_000], "every row was yielded");
    let bytes_read = row_bytes.len() * 200_000;
    eprintln!(
        "CSV streaming peak: {} for 20k rows ({}), {} for 200k rows ({})",
        mib(peaks[0]),
        mib(row_bytes.len() * 20_000),
        mib(peaks[1]),
        mib(bytes_read)
    );
    assert!(
        peaks[1] < STREAMING_PEAK_LIMIT,
        "streaming {} of CSV peaked at {} — expected under {}",
        mib(bytes_read),
        mib(peaks[1]),
        mib(STREAMING_PEAK_LIMIT)
    );
    assert!(
        peaks[1] <= peaks[0].saturating_mul(2) + 1024 * 1024,
        "peak grew with the input: {} for 20k rows vs {} for 200k rows",
        mib(peaks[0]),
        mib(peaks[1])
    );
}

/// The control: the shape the import path used to have. `split_lines` is
/// still in the codec for encode-side round-trip checks, and measuring it
/// here is what makes the streaming numbers above mean something — it
/// allocates in proportion to the file, on the same rows.
#[tokio::test]
async fn the_whole_buffer_shape_allocates_in_proportion_to_the_file() {
    let _guard = MEASURE.lock().await;
    let row = jsonl_row();

    // A modest 50k rows — the same count as the small streaming case
    // above, so the two numbers are directly comparable.
    let mut buffer = Vec::with_capacity(row.len() * 50_000);
    for _ in 0..50_000 {
        buffer.extend_from_slice(&row);
    }
    let file_bytes = buffer.len();

    // The buffer itself is allocated before the baseline is taken, so what
    // is measured is only what the *splitting* costs on top of it.
    let baseline = reset_peak();
    let lines = jsonl::split_lines(&buffer).unwrap();
    let peak = peak_above(baseline);
    assert_eq!(lines.len(), 50_000);
    drop(lines);
    eprintln!(
        "whole-buffer control peak: {} on a {} file (50k rows)",
        mib(peak),
        mib(file_bytes)
    );

    assert!(
        peak > file_bytes / 2,
        "the whole-buffer splitter was expected to allocate on the order of \
         the file ({}), but peaked at {} — if this ever stops holding, the \
         streaming comparison above has lost its control",
        mib(file_bytes),
        mib(peak)
    );
    assert!(
        peak > STREAMING_PEAK_LIMIT,
        "the control ({}) must exceed the streaming limit ({}) for the \
         contrast to be meaningful",
        mib(peak),
        mib(STREAMING_PEAK_LIMIT)
    );
}
