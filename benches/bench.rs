use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use mvar::Mvar;
use std::sync::mpsc::sync_channel;

fn bench_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("new");
    group.bench_function(BenchmarkId::new("mvar", 1), |b| {
        b.iter(|| {
            let _mvar = Mvar::<()>::empty();
        })
    });
    group.bench_function(BenchmarkId::new("mpsc", 1), |b| {
        b.iter(|| {
            let (_s, _r) = sync_channel::<()>(1);
        })
    });
    group.bench_function(BenchmarkId::new("crossbeam_channel", 1), |b| {
        b.iter(|| {
            let (_s, _r) = crossbeam_channel::bounded::<()>(1);
        })
    });
    group.finish();
}

fn bench_put_take_once(c: &mut Criterion) {
    let mvar = Mvar::empty();
    let (mpsc_send, mpsc_recv) = sync_channel(1);
    let (crossbeam_send, crossbeam_recv) = crossbeam_channel::bounded(1);
    let mut group = c.benchmark_group("put-take");
    group.bench_function(BenchmarkId::new("mvar", 1), |b| {
        b.iter(|| {
            mvar.put(()).unwrap();
            mvar.take().unwrap();
        })
    });
    group.bench_function(BenchmarkId::new("mpsc", 1), |b| {
        b.iter(|| {
            mpsc_send.send(()).unwrap();
            mpsc_recv.recv().unwrap()
        })
    });
    group.bench_function(BenchmarkId::new("crossbeam_channel", 1), |b| {
        b.iter(|| {
            crossbeam_send.send(()).unwrap();
            crossbeam_recv.recv().unwrap();
        })
    });
    group.finish();
}

criterion_group!(benches, bench_new, bench_put_take_once);
criterion_main!(benches);
