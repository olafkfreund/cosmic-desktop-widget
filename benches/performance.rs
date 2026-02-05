// Performance benchmarks for cosmic-desktop-widget
//
// Run with: cargo bench
// View results in: target/criterion/report/index.html

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

// Import the library components
use cosmic_desktop_widget::{
    widget::ClockWidget,
    update::UpdateScheduler,
    theme::Theme,
};

/// Benchmark clock widget update performance
fn bench_clock_update(c: &mut Criterion) {
    let mut clock = ClockWidget::new("24h", true, false);

    c.bench_function("clock_update", |b| {
        b.iter(|| {
            clock.update();
            black_box(clock.time_str())
        })
    });
}

/// Benchmark clock time string generation
fn bench_clock_time_string(c: &mut Criterion) {
    let clock = ClockWidget::new("24h", true, false);

    let mut group = c.benchmark_group("clock_time_string");

    // Benchmark the clone version (current API)
    group.bench_function("clone", |b| {
        b.iter(|| black_box(clock.time_string()))
    });

    // Benchmark the borrow version (optimized API)
    group.bench_function("borrow", |b| {
        b.iter(|| black_box(clock.time_str()))
    });

    group.finish();
}

/// Benchmark update scheduler
fn bench_update_scheduler(c: &mut Criterion) {
    let mut scheduler = UpdateScheduler::new(
        Duration::from_secs(1),
        Duration::from_secs(600),
    );

    c.bench_function("scheduler_check_updates", |b| {
        b.iter(|| black_box(scheduler.check_updates()))
    });
}

/// Benchmark time_until_next_update calculation
fn bench_time_until_next_update(c: &mut Criterion) {
    let scheduler = UpdateScheduler::new(
        Duration::from_secs(1),
        Duration::from_secs(600),
    );

    c.bench_function("scheduler_time_until_next", |b| {
        b.iter(|| black_box(scheduler.time_until_next_update()))
    });
}

/// Benchmark theme color conversion
fn bench_theme_operations(c: &mut Criterion) {
    let theme = Theme::cosmic_dark();

    c.bench_function("theme_background_with_opacity", |b| {
        b.iter(|| black_box(theme.background_with_opacity()))
    });
}

/// Benchmark multiple clock formats
fn bench_clock_formats(c: &mut Criterion) {
    let mut group = c.benchmark_group("clock_formats");

    for (name, format, show_seconds) in [
        ("24h_with_seconds", "24h", true),
        ("24h_no_seconds", "24h", false),
        ("12h_with_seconds", "12h", true),
        ("12h_no_seconds", "12h", false),
    ] {
        let clock = ClockWidget::new(format, show_seconds, false);
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(),
            |b, _| {
                b.iter(|| black_box(clock.time_str()))
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_clock_update,
    bench_clock_time_string,
    bench_update_scheduler,
    bench_time_until_next_update,
    bench_theme_operations,
    bench_clock_formats,
);

criterion_main!(benches);
