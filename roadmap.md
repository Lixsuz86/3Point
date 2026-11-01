# 3Point: Proje Yol Haritası (Roadmap)

> **Vizyon:** 3Point; 3D modelleme, iskelet/karakter animasyonu ve gerçek zamanlı oyun motoru runtime'ını tek bir çatı altında toplayan, %100 Rust ile yazılmış, modern, modüler ve yüksek performanslı bir dijital içerik üretim (DCC) ve oyun geliştirme stüdyosudur.

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
*Hedef: Düşük seviyeli GPU yönetimi, donanım hızlandırmalı PBR boru hattı ve esnek bir Render Graph inşası.*

- [ ] **Grafik Katmanı Soyutlaması:**
  - Modern Vulkan (`ash`) veya taşınabilir Vulkan/Metal/DX12 soyutlaması (`wgpu`).
  - Cihaz başlatma, swapchain yönetimi ve present senkronizasyonu.
- [ ] **Render Graph (Frame Graph) Mimarisi:**
  - Render geçişlerinin (passes) yönlendirilmiş asiklik çizge (DAG) olarak derlenmesi.
  - Kaynakların (Texture, Buffer) otomatik bellek yeniden kullanımı ve senkronizasyon bariyerleri.
- [ ] **PBR Render Boru Hattı:**
  - Cook-Torrance mikro yüzey BRDF (Metallic-Roughness iş akışı).
  - HDR Skybox ve Image-Based Lighting (IBL) / Irradiance hesaplamaları.
  - Gölgelendirme: Cascaded Shadow Maps (CSM) ve yumuşak gölge filtreleme (PCF).
- [ ] **Geliştirici & Hata Ayıklama Görselleştirmeleri:**
  - Grid düzlemi, koordinat eksenleri, bounding box'lar ve kamera frustum tel kafesleri.

---

### Faz 2: 3D Modelleme Katmanı (`tp_mesh`)
*Hedef: Blender/Maya benzeri hassas, topolojik olarak geçerli poligon manipülasyon çekirdeği.*

- [ ] **Yarım-Kenar (Half-Edge / DCEL) Veri Yapısı:**
  - Manifold mesh temsili: `Vertex`, `HalfEdge`, `Edge`, `Face`.
  - Sabit zamanlı komşuluk gezintisi (Vertex-to-Face, Edge-to-Face sorguları).
  - Standart üçgen/dörtgen (Tri/Quad) indeksli buffer'lar ile çift yönlü dönüştürücü.
- [ ] **Temel Geometri Üreticileri (Primitives):**
  - Parametrik Küp, Küre (UV ve Icosphere), Silindir, Düzlem, Torus.
- [ ] **Etkileşimli Modelleme Operatörleri:**
  - **Extrude:** Yüzey ve kenar ekstrüzyonu (normal doğrultulu öteleme).
  - **Inset:** Yüzey içi yeni poligon üretimi.
  - **Bevel/Chamfer:** Kenar ve köşe pah kırma.
  - **Loop Cut & Slide:** Kenar döngüsü tespiti ve bölünmesi.
  - **Catmull-Clark Yüzey Alt Bölümleme (Subdivision Surfaces):** Dinamik seviyeli yumuşatma.
- [ ] **Geri Al / Yinele (Undo/Redo) Altyapısı:**
  - Command Pattern tabanlı bellek dostu topolojik delta/state depolama.
- [ ] **Hassas Seçim (Ray-Picking):**
  - Ekrandan 3D uzaya fare ışını fırlatma.
  - BVH (Bounding Volume Hierarchy) ile milyonlarca poligon arasından Vertex, Edge, Face seçimi.

---

### Faz 3: UI/UX & Editör Katmanı (`tp_ui`)
*Hedef: Akıcı, modern, yerleştirilebilir (dockable) panel yapısıyla tümleşik DCC stüdyo arayüzü.*

- [ ] **`egui` & `egui_dock` Entegrasyonu:**
  - Panel yerleşim motoru: Sekmeli, bölünebilir, serbest bırakılabilir arayüz.
- [ ] **Sahne Viewport Entegrasyonu:**
  - Özel doku hedefli framebuffer renderlama (Off-screen rendering to egui image).
  - Akıcı kamera kontrolleri (Orbit, Pan, Zoom, Free-Fly, İzometrik/Ortografik bakış açıları).
- [ ] **Manipülasyon Gizmo'ları:**
  - 3D Taşıma (Translate), Döndürme (Rotate), Ölçekleme (Scale) eksenleri.
  - Grid Snapping ve Vertex Snapping mantığı.
- [ ] **Temel Editör Panelleri:**
  - **Outliner (Sahne Hiyerarşisi):** Nesne ağacı, parent-child ilişkileri.
  - **Inspector (Özellik Paneli):** Transform, materyal, mesh ve bileşen düzenleyicisi.
  - **Asset Browser:** Model ve doku varlıklarının önizleme listesi.

---

### Faz 4: Karakter & Nesne Animasyon Sistemi (`tp_anim`)
*Hedef: Kemik iskeletleri, anahtar kareler, interpolasyon ve ileri seviye kinematik.*

- [ ] **İskelet & Armatür (Skeletal Hierarchy):**
  - Kemik (Bone) hiyerarşisi, Local/Model uzayı dönüşüm matrisleri, Bind Pose.
- [ ] **Deri Kaplama (GPU Skeletal Skinning):**
  - Ağırlık matrisleri (Her köşe için en fazla 4-8 kemik ağırlığı).
  - Dual Quaternion ve Linear Blend Skinning (LBS) compute/vertex shader uygulaması.
- [ ] **Zaman Çizelgesi & Eğri Editörü (Timeline & Graph Editor):**
  - Dopesheet (Anahtar kare işaretleme, taşıma).
  - Bezier eğrileriyle f-curve interpolasyonu (Linear, Step, Smooth cubic).
- [ ] **Ters Kinematik (IK Çözücüleri):**
  - Two-Bone IK (Karakter bacak ve kolları için).
  - FABRIK (Forward And Backward Reaching Inverse Kinematics) omurga/uzuv çözümü.
- [ ] **Morph Targets / Blendshapes:** Yüz mimikleri ve detay deformasyonları için ağırlıklandırılmış hedef mesh'ler.

---

### Faz 5: Oyun Motoru Runtime & ECS (`tp_scene`, `tp_physics`)
*Hedef: Modellemeden bağımsız canlı oyun mantığı, bileşen tabanlı sahne ve fizik simülasyonu.*

- [ ] **Varlık Bileşen Sistemi (ECS):**
  - Yüksek verimli arketip tabanlı veri yönelimli mimari (`hecs` veya `bevy_ecs`).
  - Temel Bileşenler: `Transform`, `MeshRenderer`, `Camera`, `Light`, `RigidBody`, `Collider`.
- [ ] **Fizik Entegrasyonu (`rapier3d`):**
  - Statik, Dinamik ve Kinematik gövdeler.
  - Mesh Collider, Convex Hull, Kapsül/Kutu ilkel çarpıştırıcılar.
  - Karakter kontrolcüsü (Kinematic Character Controller).
- [ ] **Varlık Yönetimi (Asset Pipeline):**
  - glTF 2.0 / GLB yükleme ve dışa aktarma (Mesh, Skeletal, Animation, PBR Materyalleri).
  - Motorun kendi optimize edilmiş ikili (binary) formatı (`.3p`).
- [ ] **Canlı Oynatma (Play/Pause/Step Mode):**
  - Editör modundan sıfır maliyetle oyun simülasyonuna geçiş; sahne durumunun bellekte dondurulması/geri yüklenmesi.

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

