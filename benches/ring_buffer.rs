use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bounded_channel(
    tx: &crossbeam::channel::Sender<String>,
    rx: &crossbeam::channel::Receiver<String>,
    s: String,
) {
    let (tx, rx) = crossbeam::channel::bounded::<String>(10);
    for i in 0..1000 {
        if tx.is_full() {
            let _ = rx.recv().unwrap();
        }
        tx.send(s.clone()).unwrap();
    }
}

fn unbounded_channel(
    tx: &crossbeam::channel::Sender<String>,
    rx: &crossbeam::channel::Receiver<String>,
    s: String,
) {
    let (tx, rx) = crossbeam::channel::unbounded::<String>();
    for i in 0..1000 {
        if tx.len() == 10 {
            let _ = rx.recv().unwrap();
        }
        tx.send(s.clone()).unwrap();
    }
}

fn array_queue(queue: &mut crossbeam::queue::ArrayQueue<String>, s: String) {
    for i in 0..1000 {
        queue.push(s.clone());
    }
}

fn ring_channel(tx: &mut ring_channel::RingSender<String>, s: String) {
    for i in 0..1000 {
        tx.send(s.clone()).unwrap();
    }
}
fn bench_ring_buffer(c: &mut Criterion) {
    let (mut tx, mut rx) = ring_channel::ring_channel(std::num::NonZeroUsize::new(100).unwrap());
    let (bounded_tx, bounded_rx) = crossbeam::channel::bounded::<String>(10);
    let (unbounded_tx, unbounded_rx) = crossbeam::channel::unbounded::<String>();
    let mut queue = crossbeam::queue::ArrayQueue::new(10);

    let mut group = c.benchmark_group("ring buffer");
    group.bench_function("bounded_channel", |b| {
        b.iter(|| bounded_channel(&bounded_tx, &bounded_rx, String::from("fuck")));
    });
    group.bench_function("unbounded_channel", |b| {
        b.iter(|| unbounded_channel(&unbounded_tx, &unbounded_rx, String::from("fuck")));
    });
    group.bench_function("array_queue", |b| {
        b.iter(|| array_queue(&mut queue, String::from("fuck")));
    });
    group.bench_function("ring_channel", |b| {
        b.iter(|| ring_channel(&mut tx, String::from("fuck")));
    });
    group.finish();
}

criterion_group!(benches, bench_ring_buffer);
criterion_main!(benches);
