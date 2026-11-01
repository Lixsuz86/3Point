# 3Point: Yapay Zeka Geliştirici Kuralları ve Kod Standartları (`ai_rules.md`)

> **Amaç:** Bu doküman, **3Point** projesinin geliştirilmesinde yapay zeka ve insan geliştirici arasındaki iş birliğini, mimari tutarlılığı, kod kalitesini ve sistem güvenliğini garanti altına alan temel kuralları belirler. Projede üretilen her satır Rust kodu bu yönergelere istisnasız uymak zorundadır.

---

## 1. Mimari Prensipler ve Dil Yaklaşımı (Idiomatic Rust)

### 1.1. Data-Oriented Design (DOD) Önceliği
* **OOP Tarzı Düğümler Yasaktır:** `Rc<RefCell<Node>>`, `Arc<Mutex<Tree>>` gibi derin, iç içe geçmiş referans zincirleri içeren nesne yönelimli anti-pattern'lar kesinlikle kullanılmayacaktır. Bu yapılar bellek parçalanmasına (fragmentation), cache-miss patlamalarına ve kilit (deadlock) risklerine yol açar.
* **Handle ve İndeks Tabanlı Erişim:** Varlıklar, mesh elemanları ve sahne düğümleri doğrudan işaretçi yerine; `SlotMap`, `generational-arena` veya ardışık bellek dizilerindeki (`Vec<T>`) benzersiz **Entity ID / Index** yapıları üzerinden yönetilecektir.
* **Bellek Yerleşimi (Memory Layout):** Sık döngüye giren sıcak veriler (Transform, Bounding Box vb.) için Structure of Arrays (SoA) veya düz, önbellek dostu (L1/L2 cache-friendly) `#[repr(C)]` / `#[repr(align(16))]` veri yerleşimleri tercih edilmelidir.

### 1.2. Sıfır Maliyetli Soyutlama (Zero-Cost Abstractions)
* **Gereksiz Tahsislerden (Heap Allocations) Kaçının:**
  * Render ve animasyon döngüsü gibi her kare (per-frame) çalışan sıcak kod yollarında `Vec::new()`, `clone()`, `Box::new()` veya `String` tahsisatı yapılmayacaktır.
  * Sabit boyutlu veya küçük koleksiyonlar için `ArrayVec`, `SmallVec` veya yığın (stack) dilimleri (`&[T]`) kullanılacaktır.
  * `Cow<'a, T>`, `&str` ve borçlanma (borrowing) mekanizması azami ölçüde kullanılacaktır.
* **Statik Gönderim (Static Dispatch):** `dyn Trait` yerine generic türler ve trait sınırları (`impl Trait`) kullanılarak derleme zamanında monomorfizasyon sağlanmalıdır. Dinamik gönderim yalnızca açıkça plugin sistemleri veya heterojen heterojen koleksiyon gereksinimlerinde sınırlandırılmalıdır.

---

## 2. Bellek Güvenliği ve `unsafe` Politikası

### 2.1. Katı `unsafe` Kısıtlaması
* `unsafe` Rust kullanımı bir kolaylık aracı değil, yalnızca en son çaredir.
* `unsafe` blokları yalnızca şu durumlarda kabul edilebilir:
  1. Vulkan C-FFI API çağrıları (`ash` veya yerel platform fonksiyonları).
  2. Donanım seviyesinde SIMD intrinsiği kullanımı (eğer güvenli intrinsik sarmalayıcıları yetersizse).
  3. Profilleme ile kanıtlanmış, sıcak döngülerdeki sınır kontrollerinin (bounds checks) kontrollü şekilde bypass edilmesi.

### 2.2. Zorunlu `// SAFETY:` Açıklaması
Her bir `unsafe` bloğu veya fonksiyonunun hemen üstünde, o bloğun neden tanımsız davranışa (Undefined Behavior) yol açmayacağını ispatlayan bir `// SAFETY:` dokümantasyonu bulunmalıdır:

```rust
// SAFETY:
// 1. `buffer_ptr` null değildir ve geçerli bir `Vertex` dizisini işaret etmektedir.
// 2. `len * size_of::<Vertex>()` bayt uzunluğundaki bellek alanı geçerlidir.
// 3. Eşzamanlı okuma/yazma çakışması yoktur; buffer ömrü `device` ile sınırlıdır.
unsafe {
    ash_device.cmd_bind_vertex_buffers(cmd, 0, &[buffer], &[offset]);
}
```

### 2.3. Ham İşaretçilerin Kapsüllenmesi (Encapsulation)
* `unsafe` kod asla genel API'ye sızdırılmamalıdır.
* Ham işaretçiler (`*const T`, `*mut T`) içeren tüm modüller, dışarıya tamamen güvenli (safe) bir Rust arayüzü sunan RAII tipi sarmalayıcılarla (smart wrappers) paketlenmelidir.

---

## 3. Grafik ve Sistem Mühendisliği Kuralları

### 3.1. GPU Belleği ve Senkronizasyon
* **Hizalama (Alignment) Standartları:** Shader uniform ve storage buffer'larında `std140` ve `std430` hizalama kurallarına tam uyum sağlanmalıdır. Veri yapıları `#[repr(C, align(16))]` gibi açık direktiflerle işaretlenmelidir.
* **Kaynak Ömrü (Resource Lifetimes):** Bir GPU kaynağı (Buffer, Image, Pipeline) henüz bir komut tamponu (command buffer) tarafından işleniyorken CPU tarafından yok edilemez. GPU nesneleri için Frame-in-Flight tabanlı ring buffer ve çöp toplama (deferred disposal) mekanizması uygulanmalıdır.

### 3.2. Eşzamanlılık ve İş Parçacığı Güvenliği
* Uzun süreli kilitlenmelerden kaçınmak için standart kütüphane kilitleri yerine `parking_lot::Mutex` ve `parking_lot::RwLock` kullanılmalıdır.
* Render döngüsünün ana iş parçacığı (main thread) asla arka plan dosya yükleme veya karmaşık CSG/mesh hesaplamaları için bloke edilmemelidir. Bu işler `rayon` veya bir kanal (`crossbeam-channel`) üzerinden arka plan iş havuzuna yönlendirilmelidir.

### 3.3. Düşük Donanım Uyumluluğu ve Kaynak Yönetimi
* **Geniş GPU Uyumluluğu:** Yalnızca üst düzey kartlarda bulunan donanımsal Ray Tracing veya özel Vulkan 1.3 uzantılarına bağımlı kalınmamalıdır. Çekirdek özellikler `wgpu::Limits::downlevel_webgl2_defaults()` veya temel masaüstü limitleri seviyesinde de çalışabilir olmalıdır.
* **Düşük VRAM Ayak İzi:** Varsayılan doku ve geometri boyutları akıllıca sınırlandırılmalı; düşük özellikli entegre kartlar (Intel HD/UHD) göz önünde bulundurularak VRAM tüketimi minimumda tutulmalıdır.
* **CPU Yükünü Hafifletme:** Zayıf çift çekirdekli işlemcilerde dahi UI ve viewport tepkiselliğini korumak adına gereksiz polling ve boş döngülerden (busy-waiting) kaçınılmalıdır.

---

## 4. Hata Yönetimi (Error Handling) ve Savunmacı Kodlama

### 4.1. Panik Kullanımı Yasaktır
* Kütüphane (`crates/*`) kodları içerisinde `panic!`, `unwrap()` veya `expect()` kullanımı yasaktır.
* Tüm potansiyel hatalar zenginleştirilmiş hata türleriyle (`thiserror`) dışa aktarılmalı ve `Result<T, E>` yapısıyla ele alınmalıdır:

```rust
#[derive(thiserror::Error, Debug)]
pub enum MeshError {
    #[error("Geçersiz yüzey topolojisi: En az 3 köşe gereklidir, bulunan: {0}")]
    DegenerateFace(usize),
    #[error("Manifold olmayan kenar tespit edildi: {0:?}")]
    NonManifoldEdge(EdgeId),
}
```

### 4.2. Değişmezlerin Doğrulanması (Invariants)
* Geliştirme esnasında algoritma doğruluğunu sınamak için `debug_assert!` ve `debug_assert_eq!` makrolarından yararlanılmalıdır. Bu kontroller Release derlemelerinde sıfır çalışma zamanı maliyetine sahiptir.

---

## 5. Kod Kalitesi, Standartlar ve Test

### 5.1. Kod Formatı ve Statik Analiz
* Kod, standart `rustfmt` kurallarına uymalıdır.
* Clippy analizinden tek bir uyarı bile alınmamalıdır:
  ```bash
  cargo clippy --workspace --all-targets -- -D warnings
  ```
* İzin verilmeyen pratikler:
  * Kullanılmayan değişken veya içe aktarmalar (`#[allow(unused)]` izinsiz bırakılmamalıdır).
  * Karmaşık tip adları için gereksiz kısaltmalar yapılmamalı, anlamlı adlar seçilmelidir.

### 5.2. Dokümantasyon
* Tüm genel (`pub`) yapılar, enumlar, fonksiyonlar ve traitler eksiksiz Rustdoc (`///`) ile belgelenmelidir.
* Fonksiyon açıklamalarında `Arguments`, `Returns` ve fırlatılabilecek hatalar (`# Errors`) açıkça listelenmelidir.

### 5.3. Test Kapsamı
* `tp_math` ve `tp_mesh` gibi deterministik algoritmalar içeren tüm çekirdek kütüphaneler kapsamlı birim testlerine (`#[test]`) sahip olmalıdır.
* Topolojik mesh operasyonları (Extrude, Inset, Subdivide) için manifold koruma ve sınır testleri zorunludur.

---

## 6. Yapay Zekanın Kod Üretim Standartları

Yapay zeka ortak geliştirici olarak kod yazarken şu kuralları uygular:

1. **Eksiksiz ve Derlenebilir Kod:** Yarım bırakılmış, `// TODO: burayı sen tamamla` şeklinde geçiştirilmiş veya sözde kod (pseudo-code) içeren çözümler yerine; eksiksiz, derlenebilir ve tür kontrolünden geçmiş kod parçaları sunar.
2. **Crate Sınırlarına Saygı:** Yapay zeka, bir modülü geliştirirken ait olduğu crate'in sınırlarını ve bağımlılıklarını korur. Örneğin UI katmanından doğrudan düşük seviyeli Vulkan komutları çağrılmaz; `tp_gfx` arayüzü kullanılır.
3. **Sürüm Uyumluluğu:** 2024 ve sonrası modern Rust ekosistemindeki güncel crate sürümleri (örn. `wgpu` 23+, `egui` 0.29+, `glam` 0.29+, `rapier3d` 0.22+) standart kabul edilerek kodlama yapılır; eski/kullanımdan kaldırılmış (deprecated) API'ler önerilmez.
4. **Mimari Gerekçelendirme:** Yapay zeka, karmaşık mimari kararlar önerdiğinde (örneğin bellek yerleşimi veya algoritma tercihi) bunun performans ve güvenlik gerekçelerini kısaca açıklar.

