//! 3Point Modüler Render Çizgesi (Render Graph / Frame Graph).

use crate::error::GfxError;
use ash::{vk, Device};

/// Çizim geçişi türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PassType {
    /// 3D Geometri ve Mesh modelleri (Z-Buffer yazımı aktif).
    Geometry = 0,
    /// Sonsuz zemin ızgarası.
    Grid = 1,
    /// 3D Çizgi, tel kafes ve hata ayıklama çizimleri.
    DebugLines = 2,
    /// 2D Kullanıcı Arayüzü (egui vb.) en üst katman.
    OverlayUi = 3,
}

/// Render geçişinin yürütme fonksiyonu türü.
pub type PassExecuteFn<'a> = Box<dyn FnOnce(&Device, vk::CommandBuffer) -> Result<(), GfxError> + 'a>;

/// Render geçiş düğümü.
pub struct RenderPassNode<'a> {
    pub name: &'static str,
    pub pass_type: PassType,
    pub execute_fn: PassExecuteFn<'a>,
}

/// Çizim adımlarını düzenleyen ve yürüten yönlendirilmiş render grafiği.
pub struct RenderGraph<'a> {
    nodes: Vec<RenderPassNode<'a>>,
}

impl<'a> Default for RenderGraph<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> RenderGraph<'a> {
    /// Boş bir render grafiği başlatır.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(8),
        }
    }

    /// Grafiğe yeni bir yürütülebilir render adımı ekler.
    pub fn add_pass<F>(&mut self, name: &'static str, pass_type: PassType, execute: F)
    where
        F: FnOnce(&Device, vk::CommandBuffer) -> Result<(), GfxError> + 'a,
    {
        self.nodes.push(RenderPassNode {
            name,
            pass_type,
            execute_fn: Box::new(execute),
        });
    }

    /// Geçişleri mantıksal render sırasına göre sıralar ve komut tamponuna kaydeder.
    pub fn execute(mut self, device: &Device, cmd: vk::CommandBuffer) -> Result<(), GfxError> {
        // Geçişleri öncelik sıralamasına göre sırala (Geometry -> Grid -> Debug -> UI)
        self.nodes.sort_by_key(|n| n.pass_type);

        for node in self.nodes {
            (node.execute_fn)(device, cmd)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_graph_pass_ordering() {
        let mut graph = RenderGraph::new();

        graph.add_pass("UI Pass", PassType::OverlayUi, |_dev, _cmd| Ok(()));
        graph.add_pass("Geometry Pass", PassType::Geometry, |_dev, _cmd| Ok(()));
        graph.add_pass("Grid Pass", PassType::Grid, |_dev, _cmd| Ok(()));

        graph.nodes.sort_by_key(|n| n.pass_type);

        assert_eq!(graph.nodes[0].pass_type, PassType::Geometry);
        assert_eq!(graph.nodes[1].pass_type, PassType::Grid);
        assert_eq!(graph.nodes[2].pass_type, PassType::OverlayUi);
    }
}
