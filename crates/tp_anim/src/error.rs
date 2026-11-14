//! 3Point Animasyon Motoru Hata Türleri (`AnimError`).

use thiserror::Error;

/// Animasyon, iskelet ve kinematik işlemleri sırasında oluşabilecek hatalar.
#[derive(Debug, Error, PartialEq)]
pub enum AnimError {
    #[error("Geçersiz kemik ID'si: {0}")]
    InvalidBoneId(u16),

    #[error("Kemik hiyerarşisinde döngü tespit edildi: Kemik {0}")]
    CyclicHierarchy(u16),

    #[error("Kemik dizisi topolojik sıralı değil: Ebeveyn {parent} çocuk {child} kemiğinden sonra tanımlanmış")]
    NonTopologicalOrder { parent: u16, child: u16 },

    #[error("Eksik veya eşleşmeyen pose tamponu: Beklenen {expected}, Verilen {actual}")]
    PoseBufferMismatch { expected: usize, actual: usize },

    #[error("IK çözülemedi: {0}")]
    IkSolverFailed(String),

    #[error("Animasyon klibinde sıfır veya negatif süre: {0}")]
    InvalidClipDuration(f32),
}

