//! 3Point Çekirdek Hata Türleri.

#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("İş parçacığı havuzu başlatma hatası: {0}")]
    ThreadPoolError(#[from] rayon::ThreadPoolBuildError),

    #[error("G/Ç (I/O) Hatası: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Bilinmeyen çekirdek hatası: {0}")]
    Unknown(String),
}

