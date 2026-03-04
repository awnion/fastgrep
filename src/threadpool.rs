use std::thread::JoinHandle;

/// A simple fixed-size pool of named OS threads.
///
/// Uses [`std::thread::Builder`] internally so every thread receives
/// a descriptive name (required by project lint rules).
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::AtomicUsize;
/// use std::sync::atomic::Ordering;
///
/// use fastgrep::threadpool::ThreadPool;
///
/// let counter = Arc::new(AtomicUsize::new(0));
/// let c = Arc::clone(&counter);
/// let pool = ThreadPool::new(4, "worker", move || {
///     c.fetch_add(1, Ordering::Relaxed);
/// });
/// pool.join();
/// assert_eq!(counter.load(Ordering::Relaxed), 4);
/// ```
pub struct ThreadPool {
    handles: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    /// Spawns `count` threads, each running a clone of `f`.
    ///
    /// Threads are named `"{name_prefix}-0"`, `"{name_prefix}-1"`, etc.
    ///
    /// # Panics
    ///
    /// Panics if the OS refuses to create a thread.
    pub fn new<F>(count: usize, name_prefix: &str, f: F) -> Self
    where
        F: Fn() + Send + Clone + 'static,
    {
        let handles = (0..count)
            .map(|i| {
                let f = f.clone();
                std::thread::Builder::new()
                    .name(format!("{name_prefix}-{i}"))
                    .spawn(f)
                    .expect("failed to spawn thread")
            })
            .collect();
        Self { handles }
    }

    /// Waits for every thread in the pool to finish.
    pub fn join(self) {
        for h in self.handles {
            let _ = h.join();
        }
    }
}
