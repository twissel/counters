use counters::Counter as FastCounter;
use structopt::StructOpt;

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier,
    },
    thread::spawn,
    time::Instant,
};

trait Counter: Clone + Send + Sync + 'static {
    fn get(&self) -> u64;
    fn inc(&self);
    fn new() -> Self;
    fn name() -> &'static str;
}

#[derive(Clone)]
struct SimpleCounter(Arc<AtomicU64>);

impl Counter for SimpleCounter {
    #[inline]
    fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    #[inline]
    fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    fn new() -> Self {
        SimpleCounter(Arc::new(AtomicU64::new(0)))
    }

    fn name() -> &'static str {
        "Simple Atomic Counter"
    }
}

impl Counter for FastCounter {
    #[inline]
    fn get(&self) -> u64 {
        self.get()
    }

    #[inline(never)]
    fn inc(&self) {
        self.inc()
    }

    fn new() -> Self {
        FastCounter::new()
    }

    fn name() -> &'static str {
        "Fast Counter"
    }
}

#[inline(never)]
fn bench_counter_write_iter<C: Counter>(num_writer_threads: usize, num_increments: usize) -> f64 {
    let mut handles = Vec::new();
    let barrier = Arc::new(Barrier::new(num_writer_threads));
    let counter = C::new();

    for _ in 0..num_writer_threads {
        let counter_clone = counter.clone();
        let barrier_clone = barrier.clone();
        let handle = spawn(move || {
            barrier_clone.wait();
            let start_time = Instant::now();
            for _ in 0..num_increments {
                counter_clone.inc();
            }
            let elapsed = start_time.elapsed();
            (counter_clone.get(), elapsed.as_nanos())
        });
        handles.push(handle);
    }
    let avg = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .map(|(_, elapsed)| elapsed)
        .sum::<u128>() as f64
        / num_writer_threads as f64;
    assert_eq!(
        counter.get(),
        (num_writer_threads as u64) * (num_increments as u64)
    );
    avg
}

#[inline(never)]
fn bench_counter<C: Counter>(
    num_iterations: usize,
    num_writer_threads: usize,
    num_increments: usize,
) {
    let mut avg = 0.0;
    for _ in 0..num_iterations {
        avg += bench_counter_write_iter::<C>(num_writer_threads, num_increments)
            / num_iterations as f64;
    }

    println!(
        "Running {} threads, Avg. time: {:.2} ns, for thread to perform {} \"{}\"  increments",
        num_writer_threads, avg, num_increments,  C::name()
    );
}

#[derive(Debug, StructOpt)]
#[structopt(name = "counter_benchamark", about = "Run Counter Benchmark.")]
struct Options {
    /// Max threads
    #[structopt(short = "t", long = "max-threads")]
    max_threads: usize,

    /// Num iterations
    #[structopt(short = "i", long = "num-iterations", default_value = "20000")]
    num_iterations: usize,

    /// Num iterations
    #[structopt(short = "c", long = "num-increments", default_value = "100")]
    num_increments: usize,
}

fn main() {
    let options = Options::from_args();
    for threads in 1..options.max_threads + 1 {
        bench_counter::<SimpleCounter>(options.num_iterations, threads, options.num_increments);
        //bench_counter::<FastCounter>(options.num_iterations, threads, options.num_increments);
    }
}
