use std::fmt::Write as _;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::field::{Field, Visit};
use tracing::{Id, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

#[derive(Clone, Debug, Default)]
pub struct DebugProfiler {
    enabled: bool,
    state: Arc<Mutex<ProfileStore>>,
}

impl DebugProfiler {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            state: Arc::new(Mutex::new(ProfileStore::default())),
        }
    }

    pub fn run<R>(&self, f: impl FnOnce() -> R) -> R {
        if !self.enabled {
            return f();
        }

        let subscriber = tracing_subscriber::registry().with(DebugProfileLayer {
            state: Arc::clone(&self.state),
        });
        tracing::subscriber::with_default(subscriber, f)
    }

    pub fn print_report(&self) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let snapshot = self
            .state
            .lock()
            .expect("profiler lock poisoned")
            .snapshot();
        let mut stderr = io::stderr().lock();
        stderr.write_all(snapshot.render().as_bytes())?;
        stderr.flush()
    }
}

#[derive(Debug)]
struct DebugProfileLayer {
    state: Arc<Mutex<ProfileStore>>,
}

impl<S> Layer<S> for DebugProfileLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);

        let parent_index = attrs
            .parent()
            .and_then(|parent_id| span_index_from_context(&ctx, parent_id))
            .or_else(|| {
                if attrs.is_contextual() {
                    ctx.current_span()
                        .id()
                        .and_then(|current_id| span_index_from_context(&ctx, &current_id))
                } else {
                    None
                }
            });

        let mut state = self.state.lock().expect("profiler lock poisoned");
        let depth = parent_index
            .map(|index| state.spans[index].depth + 1)
            .unwrap_or(0);
        let span_index = state.new_span(
            attrs.metadata().name().to_owned(),
            visitor.finish(),
            parent_index,
            depth,
        );
        drop(state);

        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanIndex(span_index));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span_index) = span_index_from_context(&ctx, id) else {
            return;
        };

        self.state
            .lock()
            .expect("profiler lock poisoned")
            .enter(span_index, Instant::now());
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span_index) = span_index_from_context(&ctx, id) else {
            return;
        };

        self.state
            .lock()
            .expect("profiler lock poisoned")
            .exit(span_index, Instant::now());
    }
}

fn span_index_from_context<S>(ctx: &Context<'_, S>, id: &Id) -> Option<usize>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    ctx.span(id)
        .and_then(|span| span.extensions().get::<SpanIndex>().copied())
        .map(|span_index| span_index.0)
}

#[derive(Copy, Clone, Debug)]
struct SpanIndex(usize);

#[derive(Debug, Default)]
struct FieldVisitor {
    fields: Vec<String>,
}

impl FieldVisitor {
    fn finish(self) -> String {
        self.fields.join(", ")
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }
}

#[derive(Debug, Default)]
struct ProfileStore {
    spans: Vec<SpanRecord>,
    active: Vec<ActiveFrame>,
    next_start_order: usize,
}

impl ProfileStore {
    fn new_span(
        &mut self,
        name: String,
        fields: String,
        parent: Option<usize>,
        depth: usize,
    ) -> usize {
        let index = self.spans.len();
        self.spans.push(SpanRecord {
            name,
            fields,
            parent,
            depth,
            start_order: None,
            inclusive_duration: Duration::ZERO,
            child_duration: Duration::ZERO,
        });
        index
    }

    fn enter(&mut self, span_index: usize, now: Instant) {
        if self.spans[span_index].start_order.is_none() {
            self.spans[span_index].start_order = Some(self.next_start_order);
            self.next_start_order += 1;
        }

        self.active.push(ActiveFrame {
            span_index,
            entered_at: now,
            child_duration: Duration::ZERO,
        });
    }

    fn exit(&mut self, span_index: usize, now: Instant) {
        let Some(frame) = self.active.pop() else {
            return;
        };

        if frame.span_index != span_index {
            return;
        }

        let elapsed = now.saturating_duration_since(frame.entered_at);
        let record = &mut self.spans[span_index];
        record.inclusive_duration += elapsed;
        record.child_duration += frame.child_duration;

        if let Some(parent_frame) = self.active.last_mut() {
            parent_frame.child_duration += elapsed;
        }
    }

    fn snapshot(&self) -> ProfileSnapshot {
        let spans = self
            .spans
            .iter()
            .filter_map(|span| {
                span.start_order.map(|start_order| ProfileSnapshotSpan {
                    name: span.name.clone(),
                    fields: span.fields.clone(),
                    parent: span.parent,
                    depth: span.depth,
                    start_order,
                    inclusive_duration: span.inclusive_duration,
                    self_duration: span.inclusive_duration.saturating_sub(span.child_duration),
                })
            })
            .collect();

        ProfileSnapshot { spans }
    }
}

#[derive(Debug)]
struct SpanRecord {
    name: String,
    fields: String,
    parent: Option<usize>,
    depth: usize,
    start_order: Option<usize>,
    inclusive_duration: Duration,
    child_duration: Duration,
}

#[derive(Debug)]
struct ActiveFrame {
    span_index: usize,
    entered_at: Instant,
    child_duration: Duration,
}

#[derive(Debug, Default)]
struct ProfileSnapshot {
    spans: Vec<ProfileSnapshotSpan>,
}

impl ProfileSnapshot {
    fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "Debug timing report").expect("write to string");
        writeln!(&mut output).expect("write to string");
        self.render_chronological(&mut output);
        writeln!(&mut output).expect("write to string");
        self.render_slowest_first(&mut output);
        writeln!(&mut output).expect("write to string");
        writeln!(
            &mut output,
            "Total profiled time: {} across {} span(s)",
            format_duration(self.total_profiled_time()),
            self.spans.len()
        )
        .expect("write to string");
        output
    }

    fn render_chronological(&self, output: &mut String) {
        writeln!(output, "Chronological").expect("write to string");

        let mut roots = self
            .spans
            .iter()
            .enumerate()
            .filter(|(_, span)| {
                span.parent
                    .and_then(|parent| self.spans.get(parent))
                    .is_none()
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        roots.sort_by_key(|&index| self.spans[index].start_order);

        for index in roots {
            self.render_span_tree(index, output);
        }
    }

    fn render_span_tree(&self, index: usize, output: &mut String) {
        let span = &self.spans[index];
        let indent = "  ".repeat(span.depth);
        writeln!(
            output,
            "{}- {}{} [total={}, self={}]",
            indent,
            span.name,
            render_fields(&span.fields),
            format_duration(span.inclusive_duration),
            format_duration(span.self_duration)
        )
        .expect("write to string");

        let mut children = self
            .spans
            .iter()
            .enumerate()
            .filter(|(_, child)| child.parent == Some(index))
            .map(|(child_index, _)| child_index)
            .collect::<Vec<_>>();
        children.sort_by_key(|&child_index| self.spans[child_index].start_order);

        for child_index in children {
            self.render_span_tree(child_index, output);
        }
    }

    fn render_slowest_first(&self, output: &mut String) {
        writeln!(output, "Slowest first").expect("write to string");

        let mut indices = (0..self.spans.len()).collect::<Vec<_>>();
        indices.sort_by(|left, right| {
            let left_span = &self.spans[*left];
            let right_span = &self.spans[*right];

            right_span
                .self_duration
                .cmp(&left_span.self_duration)
                .then_with(|| {
                    right_span
                        .inclusive_duration
                        .cmp(&left_span.inclusive_duration)
                })
                .then_with(|| left_span.start_order.cmp(&right_span.start_order))
        });

        for index in indices {
            let span = &self.spans[index];
            writeln!(
                output,
                "- {}{} depth={} [self={}, total={}]",
                span.name,
                render_fields(&span.fields),
                span.depth,
                format_duration(span.self_duration),
                format_duration(span.inclusive_duration)
            )
            .expect("write to string");
        }
    }

    fn total_profiled_time(&self) -> Duration {
        self.spans
            .iter()
            .filter(|span| {
                span.parent
                    .and_then(|parent| self.spans.get(parent))
                    .is_none()
            })
            .fold(Duration::ZERO, |total, span| {
                total + span.inclusive_duration
            })
    }
}

#[derive(Debug)]
struct ProfileSnapshotSpan {
    name: String,
    fields: String,
    parent: Option<usize>,
    depth: usize,
    start_order: usize,
    inclusive_duration: Duration,
    self_duration: Duration,
}

fn render_fields(fields: &str) -> String {
    if fields.is_empty() {
        String::new()
    } else {
        format!(" {{{fields}}}")
    }
}

fn format_duration(duration: Duration) -> String {
    if duration >= Duration::from_secs(1) {
        format!("{:.3}s", duration.as_secs_f64())
    } else if duration >= Duration::from_millis(1) {
        format!("{:.3}ms", duration.as_secs_f64() * 1000.0)
    } else if duration >= Duration::from_micros(1) {
        format!("{:.3}us", duration.as_secs_f64() * 1_000_000.0)
    } else {
        format!("{}ns", duration.as_nanos())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::ProfileStore;

    #[test]
    fn nested_spans_compute_inclusive_and_self_durations() {
        let mut store = ProfileStore::default();
        let parent = store.new_span("command".to_owned(), String::new(), None, 0);
        let child = store.new_span("query".to_owned(), String::new(), Some(parent), 1);
        let started_at = Instant::now();

        store.enter(parent, started_at);
        store.enter(child, started_at + Duration::from_millis(2));
        store.exit(child, started_at + Duration::from_millis(5));
        store.exit(parent, started_at + Duration::from_millis(9));

        let snapshot = store.snapshot();

        assert_eq!(
            snapshot.spans[0].inclusive_duration,
            Duration::from_millis(9)
        );
        assert_eq!(snapshot.spans[0].self_duration, Duration::from_millis(6));
        assert_eq!(
            snapshot.spans[1].inclusive_duration,
            Duration::from_millis(3)
        );
        assert_eq!(snapshot.spans[1].self_duration, Duration::from_millis(3));
    }

    #[test]
    fn repeated_sibling_spans_keep_stable_chronological_order() {
        let mut store = ProfileStore::default();
        let root = store.new_span("command".to_owned(), String::new(), None, 0);
        let first = store.new_span("draw".to_owned(), "step=1".to_owned(), Some(root), 1);
        let second = store.new_span("draw".to_owned(), "step=2".to_owned(), Some(root), 1);
        let started_at = Instant::now();

        store.enter(root, started_at);
        store.enter(first, started_at + Duration::from_millis(1));
        store.exit(first, started_at + Duration::from_millis(2));
        store.enter(second, started_at + Duration::from_millis(3));
        store.exit(second, started_at + Duration::from_millis(4));
        store.exit(root, started_at + Duration::from_millis(5));

        let rendered = store.snapshot().render();
        let first_index = rendered.find("draw {step=1}").expect("first draw span");
        let second_index = rendered.find("draw {step=2}").expect("second draw span");

        assert!(first_index < second_index);
    }

    #[test]
    fn slowest_first_sort_is_deterministic() {
        let mut store = ProfileStore::default();
        let root = store.new_span("command".to_owned(), String::new(), None, 0);
        let fast = store.new_span("fast".to_owned(), String::new(), Some(root), 1);
        let slow = store.new_span("slow".to_owned(), String::new(), Some(root), 1);
        let started_at = Instant::now();

        store.enter(root, started_at);
        store.enter(fast, started_at + Duration::from_millis(1));
        store.exit(fast, started_at + Duration::from_millis(2));
        store.enter(slow, started_at + Duration::from_millis(3));
        store.exit(slow, started_at + Duration::from_millis(8));
        store.exit(root, started_at + Duration::from_millis(9));

        let rendered = store.snapshot().render();
        let section = rendered
            .split("Slowest first\n")
            .nth(1)
            .expect("slowest section should exist");
        let slow_index = section.find("slow").expect("slow span should exist");
        let fast_index = section.find("fast").expect("fast span should exist");

        assert!(slow_index < fast_index);
    }

    #[test]
    fn render_includes_both_sections_and_footer() {
        let mut store = ProfileStore::default();
        let root = store.new_span("command".to_owned(), "command=\"ls\"".to_owned(), None, 0);
        let started_at = Instant::now();

        store.enter(root, started_at);
        store.exit(root, started_at + Duration::from_millis(1));

        let rendered = store.snapshot().render();

        assert!(rendered.contains("Chronological"));
        assert!(rendered.contains("Slowest first"));
        assert!(rendered.contains("Total profiled time:"));
        assert!(rendered.contains("command {command=\"ls\"}"));
    }
}
