//! 3Point Paralel İş Parçacığı ve Görev Havuzu (`rayon`).

use std::sync::Arc;

/// Arka plan hesaplamaları (mesh üretimi, CSG, fizik ve asset parsing) için
/// ayrılmış, yapılandırılabilir iş parçacığı havuzu.
#[derive(Clone)]
pub struct TaskPool {
    pool: Arc<rayon::ThreadPool>,
}

impl TaskPool {
    /// Sistemdeki mantıksal çekirdek sayısına göre optimize edilmiş yeni bir görev havuzu oluşturur.
    pub fn new(num_threads: Option<usize>) -> Result<Self, rayon::ThreadPoolBuildError> {
        let mut builder = rayon::ThreadPoolBuilder::new().thread_name(|idx| format!("tp-worker-{idx}"));
        if let Some(n) = num_threads {
            builder = builder.num_threads(n);
        }

        let pool = builder.build()?;
        tracing::debug!("TaskPool oluşturuldu: {} iş parçacığı.", pool.current_num_threads());
        Ok(Self { pool: Arc::new(pool) })
    }

    /// Havuzda asenkron bir kapatma (closure) çalıştırır.
    pub fn spawn<F>(&self, func: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.spawn(func);
    }

    /// Bir koleksiyon üzerinde paralel iterasyon çalıştırır.
    pub fn install<OP, R>(&self, op: OP) -> R
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        self.pool.install(op)
    }

    /// Aktif iş parçacığı sayısını döndürür.
    #[must_use]
    pub fn num_threads(&self) -> usize {
        self.pool.current_num_threads()
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new(None).expect("Varsayılan TaskPool başlatılamadı!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_task_pool_execution() {
        let pool = TaskPool::new(Some(4)).unwrap();
        let counter = Arc::new(AtomicUsize::new(0));

        let c1 = counter.clone();
        pool.install(|| {
            c1.fetch_add(10, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
}

