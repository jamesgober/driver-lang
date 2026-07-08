//! Criterion benchmarks for the driver's hot paths: running a pipeline and
//! accumulating diagnostics in a session.
//!
//! The pipeline is fully monomorphized, so these measure the real cost of
//! threading an artifact through the stage chain — there is no dynamic dispatch to
//! amortize.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use driver_lang::{DriverError, Pipeline, Session, Stage};

/// Doubles every element in place-by-value.
struct Double;
impl Stage<()> for Double {
    type Input = Vec<i64>;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str {
        "double"
    }
    fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>) -> Result<Vec<i64>, DriverError> {
        Ok(input.into_iter().map(|x| x.wrapping_mul(2)).collect())
    }
}

/// Adds one to every element.
struct Increment;
impl Stage<()> for Increment {
    type Input = Vec<i64>;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str {
        "increment"
    }
    fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>) -> Result<Vec<i64>, DriverError> {
        Ok(input.into_iter().map(|x| x.wrapping_add(1)).collect())
    }
}

/// Sums a list to a scalar — the type-changing terminal stage.
struct Sum;
impl Stage<()> for Sum {
    type Input = Vec<i64>;
    type Output = i64;
    fn name(&self) -> &'static str {
        "sum"
    }
    fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>) -> Result<i64, DriverError> {
        Ok(input.iter().fold(0i64, |a, &b| a.wrapping_add(b)))
    }
}

fn bench_pipeline_run(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_run");
    for len in [16usize, 256, 4096] {
        let input: Vec<i64> = (0..len as i64).collect();
        group.throughput(Throughput::Elements(len as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &input, |b, input| {
            let mut driver = Pipeline::new(Double).then(Increment).then(Sum);
            b.iter(|| {
                let mut session = Session::new(());
                let out = driver
                    .run(black_box(input.clone()), &mut session)
                    .expect("pipeline succeeds");
                black_box(out)
            });
        });
    }
    group.finish();
}

fn bench_session_emit(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_emit");
    for count in [8usize, 128, 1024] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mut session = Session::new(());
                for _ in 0..count {
                    session.warn("a diagnostic message");
                }
                black_box(session.diagnostics().len())
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_pipeline_run, bench_session_emit);
criterion_main!(benches);
