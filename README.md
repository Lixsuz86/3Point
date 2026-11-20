# 3Point (3P) — All-in-One 3D Studio & Live Game Engine

![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?style=flat-square&logo=rust)
![Graphics](https://img.shields.io/badge/Vulkan-1.2%20(ash)-red?style=flat-square&logo=vulkan)
![UI](https://img.shields.io/badge/UI-egui%20(Immediate%20Mode)-blue?style=flat-square)
![Physics](https://img.shields.io/badge/Physics-Rapier3D-green?style=flat-square)
![Tests](https://img.shields.io/badge/Tests-33%2F33%20Passing-brightgreen?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square)

> **3Point**, 3D poligon modelleme, iskelet/karakter animasyonu ve gerçek zamanlı oyun motoru runtime'ını tek bir çatı altında birleştiren, **%100 saf Rust** ile sıfırdan yazılmış, modern ve modüler bir dijital içerik üretim (DCC) stüdyosudur.

---

## 🎯 Vizyon ve Tasarım İlkeleri

- **🌱 Düşük Donanım Dostu:** Ağır SDK bağımlılıkları ve C++ çalışma ortamı olmadan; eski nesil Intel/AMD entegre GPU'lar ve giriş seviyesi dizüstü bilgisayarlarda akıcı 60 FPS viewport deneyimi.
- **⚡ Yüksek Performanslı Rust Mimarisi:** Bellek güvenliği, sıfır maliyetli soyutlamalar (zero-cost abstractions) ve SIMD hızlandırmalı matematik.
- **🧱 Hepsi Bir Arada Entegrasyon:** Modelleme (Half-Edge / B-Rep), İskelet & Karakter Animasyonu (FK, Dual Quaternion Skinning, İki Kemikli ve FABRIK IK), Sahne & ECS (Hecs) ve 3D Fizik Simülasyonu (Rapier3D).
- **🎨 Modern ve Sade Editör:** Koyu stüdyo teması, yerleştirilebilir dock panelleri, kamera navigasyonu (Orbit, Pan, Zoom) ve anlık telemetri (FPS, Frame Time, Frustum Culling).

---

## 🏛️ Çalışma Alanı Mimarisi (Workspace Crates)

Proje, 8 bağımsız ve test edilebilir alt modülden (crates) oluşan temiz bir Cargo Workspace'tir:

```text
crates/
├── tp_math/        # SIMD hızlandırmalı vektör, matris, kuaterniyon, Frustum, Ray ve AABB matematiği
├── tp_core/        # Hata ayıklama, loglama (tracing) ve CPU/GPU zamanlama profilleri
├── tp_gfx/         # Saf Vulkan 1.2 (ash) donanım katmanı, çoklu GPU seçimi, Swapchain ve RenderPass
├── tp_mesh/        # Half-Edge DCEL topolojisi, modelleme operatörleri (Extrude, Inset, Subdiv), Wavefront OBJ ve .3P ikili formatı
├── tp_anim/        # İskelet armatürleri, Forward Kinematics, LBS/DQS skinning, Two-Bone & FABRIK IK çözücüleri, Timeline
├── tp_physics/     # Rapier3D tabanlı 3D dinamik/statik katı cisimler ve çarpıştırıcılar
├── tp_scene/       # ECS (Hecs) sahne grafiği, yaşam döngüsü (Edit/Play/Pause) ve hazır şablon sahneleri
├── tp_ui/          # Vulkan doğrudan çizimli egui stüdyo arayüzü, Outliner, Inspector, Toolbox, Timeline
└── tp_app/         # Ana masaüstü uygulaması ve koordinasyon döngüsü
```

---

## 🚀 Tamamlanan Fazlar (Phase Roadmap)

- [x] **Faz 0: Temel Altyapı & Matematik (`tp_math`, `tp_core`)** — SIMD lineer cebir, transform hiyerarşisi, frustum/ray/AABB testleri.
- [x] **Faz 1: Vulkan Render Motoru (`tp_gfx`)** — `ash` 0.38 ile Vulkan 1.2 HAL, Entegre & Ayrık GPU puanlama, Frames-in-Flight, RenderPass.
- [x] **Faz 2: 3D Modelleme Katmanı (`tp_mesh`)** — Half-Edge topoloji, parametrik primitifler (Küp, Küre, Silindir, Düzlem), Extrude, Inset, Catmull-Clark yüzey yumuşatma, Undo/Redo altyapısı, Ray-Picking.
- [x] **Faz 3: UI/UX & Editör Katmanı (`tp_ui`)** — Vulkan RenderPass içi egui entegrasyonu, Outliner, Inspector, Toolbox, üst menü, koyu tema.
- [x] **Faz 4: Karakter & Nesne Animasyon Sistemi (`tp_anim`)** — Armatür & İskelet, FK, LBS & Dual Quaternion Skinning (DQS), Two-Bone analitik IK, FABRIK omurga IK, etkileşimli Timeline ve oynatıcı.
- [x] **Faz 5: Oyun Motoru Runtime & ECS (`tp_scene`, `tp_physics`)** — ECS arketip bileşenleri, Rapier3D fiziği, Edit/Play/Pause çalışma durumları, sahne Snapshot & Restore mekanizması.
- [x] **Faz 6: İleri Seviye Optimizasyon ve MVP Sürümü** — 6-Düzlemli analitik Frustum Culling, Wavefront OBJ ve sıfır-kopyalı `.3p` ikili formatı, hazır şablon sahneler (Boş Stüdyo, Karakter, Fizik Kum Havuzu), telemetri paneli ve LTO optimize release profili.

---

## 🛠️ Kurulum ve Çalıştırma

### Gereksinimler
- **Rust 1.75+** (`rustup update stable`)
- Vulkan 1.2 uyumlu grafik sürücüsü (Tüm modern Intel HD/UHD/Iris, AMD Radeon veya NVIDIA sürücüleri)

### Testleri Çalıştırma
Tüm workspace testlerini çalıştırmak için:
```bash
cargo test --workspace
```
*(33/33 test sıfır hata ile geçmektedir.)*

### Kod Standartları ve Linter
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Stüdyoyu Başlatma
```bash
# Debug Modu
cargo run -p tp_app

# Yüksek Performanslı Release (LTO Optimize) Modu
cargo run --release -p tp_app
```

---

## 📦 Varlık İçe / Dışa Aktarma (Asset IO)

| Format | Uzantı | Açıklama |
|---|---|---|
| **Wavefront OBJ** | `.obj` | Blender, Maya, 3ds Max ve oyun motorları ile evrensel poligon ve köşe normalleri uyumluluğu. |
| **3Point Binary** | `.3p` | Ultra hızlı, sıfır kopyalı (`bytemuck`) yerel ikili format. Header, AABB sınırları, vertex ve index buffer'larını anında yükler. |

---

## 📄 Lisans
Bu proje [MIT](LICENSE) lisansı altında geliştirilmektedir.
