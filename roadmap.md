# 3Point: Proje Yol Haritası (Roadmap)

> **Vizyon:** 3Point; 3D modelleme, iskelet/karakter animasyonu ve gerçek zamanlı oyun motoru runtime'ını tek bir çatı altında toplayan, %100 Rust ile yazılmış, modern, modüler ve yüksek performanslı bir dijital içerik üretim (DCC) ve oyun geliştirme stüdyosudur.
>
> **Hedef Kitle & Donanım Profili:**
> * **Birincil Hedef:** Yeni başlayanlar, öğrenciler, bağımsız geliştiriciler ve düşük donanımlı (entegre Intel/AMD GPU'lu dizüstü bilgisayarlar, eski nesil ekran kartları) kullanıcılar. Karmaşık menüler yerine sezgisel ve hafif bir stüdyo deneyimi.
> * **İkincil Hedef:** Yüksek donanımlı sistemlerde çalışan, sıfır gecikmeli iş akışı ve ileri seviye PBR/GPU culling yeteneklerini talep eden profesyoneller.

---

## 🏛️ Mimari Çalışma Alanı (Workspace Architecture)

Proje, bağımsız derlenebilir ve test edilebilir alt modüllerden (crates) oluşan bir Cargo Workspace olarak tasarlanmıştır:

```text
3point/
├── Cargo.toml                  # Workspace manifest
├── crates/
│   ├── tp_math/                # SIMD destekli lineer cebir, dönüşüm matrisleri, kuaterniyonlar
│   ├── tp_core/                # Görev havuzu, bellek tahsisçileri, olay döngüsü, loglama
│   ├── tp_gfx/                 # Grafik Donanım Katmanı (Vulkan / wgpu soyutlaması, RenderGraph)
│   ├── tp_mesh/                # B-Rep / Half-Edge topoloji, CSG, subdiv algoritmaları
│   ├── tp_anim/                # İskelet, kemik hiyerarşisi, GPU skinning, IK çözücüleri, timeline
│   ├── tp_scene/               # ECS tabanlı sahne yapısı, uzamsal indeksleme (BVH/Octree)
│   ├── tp_physics/             # Çarpışma algılama ve dinamikler (Rapier3D entegrasyonu)
│   ├── tp_ui/                  # egui/egui_dock editör pencereleri, gizmo'lar, sahne önizlemesi
│   └── tp_app/                 # Ana masaüstü yürütülebilir dosyası ve orchestration
```

---

## 🗺️ Fazlar ve Kilometre Taşları

```
[Faz 0: Temel Altyapı] 
        │
        ▼
[Faz 1: Render Motoru Çekirdeği] ──► [Faz 2: 3D Modelleme Katmanı]
        │                                     │
        ▼                                     ▼
[Faz 3: UI & Editör Entegrasyonu] ◄── [Faz 4: Animasyon Sistemi]
        │
        ▼
[Faz 5: ECS & Oyun Motoru Runtime]
        │
        ▼
[Faz 6: MVP Sürümü & Optimizasyon]
```

---

### Faz 0: Temel Altyapı ve Çalışma Ortamı (Foundation)
*Hedef: Sağlam bir Rust geliştirme altyapısının, loglama sisteminin ve matematik kütüphanesinin kurulması.*

- [x] **Workspace Kurulumu:** Modüler Cargo workspace yapısının tanımlanması ve bağımlılık optimizasyonu.
- [x] **Matematik Kütüphanesi (`tp_math`):** 
  - `glam` veya doğrudan SIMD hızlandırmalı vektör, matris ve kuaterniyon yapıları.
  - Transform hiyerarşileri (LocalTransform, WorldTransform).
  - Ray, AABB, Frustum ve OBB geometrik testleri.
- [x] **Sistem Çekirdeği (`tp_core`):**
  - Çok kanallı asenkron loglama ve teşhis (`tracing`, `tracing-subscriber`).
  - CPU/GPU profilleme altyapısı (`tracy-client` veya `puffin`).
  - Paralel iş parçacığı havuzu (`rayon` entegrasyonu).
- [x] **Pencereleme ve Olay Döngüsü (`winit`):** DPI duyarlı çoklu platform pencere oluşturma ve OS giriş olaylarının dinlenmesi.

---

### Faz 1: Grafik & Render Altyapısı (`tp_gfx`)
*Hedef: Saf Vulkan (ash) donanım katmanı, çoklu GPU puanlama (Entegre + Ayrık kartlar), Swapchain ve Frames-in-Flight senkronizasyonu.*

- [x] **Vulkan Grafik Donanım Katmanı (`ash`):**
  - Vulkan 1.2 tabanı ile geniş donanım uyumluluğu (Eski entegre Intel/AMD GPU'lar ve modern kartlar).
  - Güvenli `Instance`, hata ayıklama doğrulama katmanları (`VK_LAYER_KHRONOS_validation`), ve platformlar arası `Surface`.
  - Akıllı GPU seçimi: Ayrık GPU önceliği ve entegre GPU desteği ile grafik/sunum kuyruk aileleri.
- [x] **Swapchain & Senkronizasyon:**
  - Dynamic Extent, sRGB renk uzayı, V-Sync/Mailbox sunum modları ve pencere boyutu değişiminde otomatik `recreate_swapchain`.
  - CPU-GPU eşzamanlılığı için 2 Frames-in-Flight semafor ve fence yönetimi.
  - Standart RenderPass ve Framebuffer boru hattı ile stüdyo koyu arka plan render döngüsü.
- [x] **Ölçeklenebilir Render Hattı (Scalable Pipeline):**
  - **Düşük Donanım / Modelleme Modu:** MatCap (Material Capture) ve temel yönsüz/Blinn-Phong aydınlatma.
  - **Studio PBR Modu:** Cook-Torrance BRDF (Metallic-Roughness), HDR IBL ve dinamik gölgeler.
- [x] **Render Graph (Frame Graph) Mimarisi:**
  - Render geçişlerinin (passes) yönlendirilmiş asiklik çizge (DAG) olarak derlenmesi.
  - Kaynakların (Texture, Buffer) otomatik bellek yeniden kullanımı ve senkronizasyon bariyerleri.
- [x] **Geliştirici & Hata Ayıklama Görselleştirmeleri:**
  - Grid düzlemi, koordinat eksenleri, bounding box'lar ve kamera frustum tel kafesleri.

---

### Faz 2: 3D Modelleme Katmanı (`tp_mesh`)
*Hedef: Blender/Maya benzeri hassas, topolojik olarak geçerli poligon manipülasyon çekirdeği.*

- [x] **Yarım-Kenar (Half-Edge / DCEL) Veri Yapısı:**
  - Manifold mesh temsili: `Vertex`, `HalfEdge`, `Edge`, `Face`.
  - Sabit zamanlı komşuluk gezintisi (Vertex-to-Face, Edge-to-Face sorguları).
  - Standart üçgen/dörtgen (Tri/Quad) indeksli buffer'lar ile çift yönlü dönüştürücü (`to_tri_mesh`).
- [x] **Temel Geometri Üreticileri (Primitives):**
  - Parametrik Küp, UV Küresi, Silindir, Düzlem ve Torus.
- [x] **Etkileşimli Modelleme Operatörleri:**
  - **Extrude:** Yüzey ve kenar ekstrüzyonu (normal doğrultulu öteleme).
  - **Inset:** Yüzey içi yeni poligon üretimi.
  - **Catmull-Clark Yüzey Alt Bölümleme (Subdivision Surfaces):** Dinamik seviyeli yumuşatma.
- [x] **Geri Al / Yinele (Undo/Redo) Altyapısı:**
  - Command Pattern tabanlı bellek dostu topolojik snapshot/delta depolama (`UndoStack`).
- [x] **Hassas Seçim (Ray-Picking):**
  - Ekrandan 3D uzaya fare ışını fırlatma.
  - BVH (Bounding Volume Hierarchy) ile poligonlar arasından $O(\log N)$ hızında Face ve Vertex seçimi.

---

### Faz 3: UI/UX & Editör Katmanı (`tp_ui`)
*Hedef: Akıcı, modern, yerleştirilebilir (dockable) panel yapısıyla tümleşik DCC stüdyo arayüzü.*

- [x] **`egui` & `egui-winit` Vulkan Entegrasyonu (`EguiVulkanRenderer`):**
  - Sıfır kopyalı vertex/index transferi, font doku atlası yükleme, scissor clipping ile doğrudan Vulkan RenderPass üzerinde GPU çizimi.
  - Özel koyu stüdyo teması (Blender 4.x & Unreal 5 estetiği, yüksek kontrast ve göz yormayan palet).
- [x] **Sahne Viewport & Giriş Yönetimi:**
  - Akıcı kamera kontrolleri (Orbit, Pan, Zoom) ve egui giriş tüketim mekanizması (`handle_event`).
  - Viewport üzerinde 3D çizim ve UI panellerinin eşzamanlı sıfır gecikmeli çalışması.
- [x] **Manipülasyon Araçları & Seçenekler:**
  - Taşıma (Translate), Döndürme (Rotate), Ölçekleme (Scale) ve Seçim araçları.
  - Grid Snapping anahtarı ve topoloji seçim modları (Vertex, Edge, Face).
- [x] **Temel Editör Panelleri:**
  - **Üst Menü Çubuğu (Top Menu Bar):** Dosya, Düzenle, Ekle (Küp, Küre, Silindir, Düzlem), Görünüm (MatCap, PBR, Wireframe) ve Stüdyo Modları (Modelleme, Animasyon, Sahne, Oyun).
  - **Sol Araç Çubuğu (Toolbox):** Hızlı erişim seçim filtreleri, transform araçları ve modelleme butonları (Extrude, Inset, Subdivide).
  - **Outliner (Sahne Hiyerarşisi):** Sahne nesneleri ağacı (Mesh, Kamera, Işık), görünürlük açma/kapatma ve odaklanma.
  - **Inspector (Özellik Paneli):** Hassas sayısal Transform (X/Y/Z Konum, Döndürme, Ölçek), anlık poligon/kenar/yüzey sayaçları ve parametrik modelleme girdileri.

---

### Faz 4: Karakter & Nesne Animasyon Sistemi (`tp_anim`)
*Hedef: Kemik iskeletleri, anahtar kareler, interpolasyon ve ileri seviye kinematik.*

- [x] **İskelet & Armatür (`Skeleton`, `Bone`):**
  - Topolojik düz dizilimli (flat-array) kemik hiyerarşisi, sıfır heap maliyetli $O(N)$ Forward Kinematics (FK).
  - Bind Pose ve GPU/CPU için ters bağlama matrisleri ($M_{skin} = M_{world} \cdot M_{bind}^{-1}$).
- [x] **Deri Kaplama (Skeletal Skinning):**
  - Köşe başına 4 kemik ağırlığı (`JointWeights`, Vulkan UBO dostu).
  - Linear Blend Skinning (LBS) ve eklem burulmalarında hacim daralmasını engelleyen Dual Quaternion Skinning (DQS).
- [x] **Zaman Çizelgesi & Eğri Motoru (Timeline & Curve):**
  - Step, Linear (Slerp) ve Hermite Cubic Spline interpolasyonu.
  - İkili arama ($O(\log K)$) ile hassas zaman örneklemesi.
  - Editör alt panelinde etkileşimli Animasyon Zaman Çizelgesi (Timeline, Play/Pause/Loop, Scrubber, FPS).
- [x] **Ters Kinematik (IK Çözücüleri):**
  - İki kemikli analitik (Closed-form) Two-Bone IK çözücüsü (mikro-saniyelik, $O(1)$ süre, sıfır iterasyon).
  - Çok eklemli omurga ve kuyruklar için açı tekilliksiz FABRIK çözücüsü.
- [x] **Animasyon Karıştırma & Oynatıcı (`blend`, `player`):**
  - Doğrusal poz harmanlama (Linear Pose Blending), kemik maskeleme (Bone Masking) ve Additive Blending.
  - Klipler arasında yumuşak geçiş (Cross-Fade Transition) ve oynatma hızı yönetimi.

---

### Faz 5: Oyun Motoru Runtime & ECS (`tp_scene`, `tp_physics`)
*Hedef: Modellemeden bağımsız canlı oyun mantığı, bileşen tabanlı sahne ve fizik simülasyonu.*

- [x] **Varlık Bileşen Sistemi (ECS - `hecs`):**
  - Düşük donanımlı sistemler ve başlangıç seviyesi PC'ler için ultra hafif, hızlı ve yüksek önbellek dostu arketip mimarisi.
  - Temel Bileşenler: `Name`, `TransformComponent`, `GlobalTransform`, `Hierarchy`, `MeshRenderer`, `CameraComponent`, `LightComponent`, `PhysicsBodyComponent`, `PhysicsColliderComponent`.
- [x] **3D Fizik Motoru Entegrasyonu (`tp_physics`, `rapier3d`):**
  - Dinamik, Statik (Fixed) ve Kinematik hareket modelleri (`RigidBodyDesc`).
  - Kutu, Küre, Kapsül, Silindir ve Üçgen Ağ (TriMesh) çarpıştırıcıları (`ColliderDesc`).
  - Yerçekimi, sürtünme, sekme (restitution), itme (impulse) ve hızlandırılmış Raycast ışın testleri.
- [x] **Sahne Grafiği & Yaşam Döngüsü (Scene Graph & Lifecycle):**
  - Stack tabanlı, sıfır heap maliyetli Parent-Child hiyerarşik dönüşüm yayılımı ($M_{global} = M_{parent} \cdot M_{child}$).
  - Canlı Motor Çalışma Durumları: `Edit`, `Play`, `Pause`.
  - Snapshot & Restore: Play moduna girildiğinde sahne dönüşümlerini donduran ve Stop edildiğinde tüm sahneyi orijinal düzenleme anına hatasız geri yükleyen güvenlik mekanizması.
- [x] **UI & Runtime Entegrasyonu:**
  - DCC editör üst menüsünde Oyun Testi modu ve canlı çalışma butonları (`▶ PLAY`, `⏸ PAUSE`, `⏹ STOP`).
  - Viewport döngüsünde 60 FPS eşzamanlı fizik ve hiyerarşi güncellemesi.

---

### Faz 6: İleri Seviye Optimizasyon ve MVP Sürümü
*Hedef: Üretim seviyesinde kararlılık, düşük bellek ayak izi ve ilk resmi dağıtım.*

- [ ] **GPU-Driven Rendering & Culling:**
  - Compute tabanlı Frustum Culling ve Donanım Oklüzyon Testleri (Hi-Z).
  - Bindless Textures / Indirect Draw çağrıları ile CPU yükünün minimize edilmesi.
- [ ] **Çoklu İş Parçacıklı Performans İnce Ayarları:**
  - Cache-friendly SoA (Structure of Arrays) dönüşümleri.
  - Tracy ile frame süresi darboğazlarının analizi (hedef: 4K 60+ FPS kararlı viewport).
- [ ] **MVP Paketi:**
  - Entegre modelleme, kemik bağlama, animasyon yapma ve sahneyi oyun modunda gezinilebilir şekilde çalıştırma demonstrasyonu.

