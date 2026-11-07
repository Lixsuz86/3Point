//! 3Point Geri Al / Yinele (Undo/Redo) Komut Mimarisi (`Command Pattern`).

use crate::{error::MeshError, mesh::HalfEdgeMesh};

/// Mesh üzerinde yürütülebilen ve geri alınabilen komut arayüzü.
pub trait MeshCommand: std::fmt::Debug {
    /// Komutun adını döner (UI'da göstermek için: Örn: "Extrude Face").
    fn name(&self) -> &'static str;

    /// Komutu mesh üzerinde uygular.
    fn execute(&mut self, mesh: &mut HalfEdgeMesh) -> Result<(), MeshError>;

    /// Yapılan işlemi geri alır ve mesh'i önceki durumuna döndürür.
    fn undo(&mut self, mesh: &mut HalfEdgeMesh) -> Result<(), MeshError>;
}

/// Mesh eylemi kapatma (closure) türü.
pub type MeshActionFn = Box<dyn FnOnce(&mut HalfEdgeMesh) -> Result<(), MeshError>>;

/// Mesh'in durum kopyasını tutarak geri almayı sağlayan evrensel anlık görüntü (Snapshot) komutu.
pub struct SnapshotMeshCommand {
    name: &'static str,
    before: Option<HalfEdgeMesh>,
    after: Option<HalfEdgeMesh>,
    action: Option<MeshActionFn>,
}

impl std::fmt::Debug for SnapshotMeshCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotMeshCommand")
            .field("name", &self.name)
            .field("before_has_mesh", &self.before.is_some())
            .field("after_has_mesh", &self.after.is_some())
            .finish()
    }
}

impl SnapshotMeshCommand {
    /// Yeni bir anlık görüntü komutu tanımlar.
    pub fn new<F>(name: &'static str, action: F) -> Self
    where
        F: FnOnce(&mut HalfEdgeMesh) -> Result<(), MeshError> + 'static,
    {
        Self {
            name,
            before: None,
            after: None,
            action: Some(Box::new(action)),
        }
    }
}

impl MeshCommand for SnapshotMeshCommand {
    fn name(&self) -> &'static str {
        self.name
    }

    fn execute(&mut self, mesh: &mut HalfEdgeMesh) -> Result<(), MeshError> {
        self.before = Some(mesh.clone());
        if let Some(action) = self.action.take() {
            action(mesh)?;
            self.after = Some(mesh.clone());
        } else if let Some(after) = &self.after {
            *mesh = after.clone();
        }
        Ok(())
    }

    fn undo(&mut self, mesh: &mut HalfEdgeMesh) -> Result<(), MeshError> {
        if let Some(before) = &self.before {
            *mesh = before.clone();
            Ok(())
        } else {
            Err(MeshError::TopologyError("Geri alınacak önceki durum bulunamadı.".into()))
        }
    }
}

/// Geri Al ve Yinele yığınlarını yöneten yapı.
#[derive(Default)]
pub struct UndoStack {
    undo_stack: Vec<Box<dyn MeshCommand>>,
    redo_stack: Vec<Box<dyn MeshCommand>>,
    max_capacity: usize,
}

impl UndoStack {
    /// Belirtilen geçmiş derinliği ile yeni bir UndoStack oluşturur.
    #[must_use]
    pub fn new(max_capacity: usize) -> Self {
        Self {
            undo_stack: Vec::with_capacity(max_capacity),
            redo_stack: Vec::with_capacity(max_capacity),
            max_capacity: max_capacity.max(1),
        }
    }

    /// Yeni bir komut yürütür ve geri alma geçmişine kaydeder (Yineleme yığını temizlenir).
    pub fn execute<C: MeshCommand + 'static>(
        &mut self,
        mut command: C,
        mesh: &mut HalfEdgeMesh,
    ) -> Result<(), MeshError> {
        command.execute(mesh)?;

        if self.undo_stack.len() >= self.max_capacity {
            self.undo_stack.remove(0);
        }

        self.undo_stack.push(Box::new(command));
        self.redo_stack.clear();
        Ok(())
    }

    /// Son işlemi geri alır.
    pub fn undo(&mut self, mesh: &mut HalfEdgeMesh) -> Result<bool, MeshError> {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(mesh)?;
            self.redo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Geri alınan son işlemi yineler.
    pub fn redo(&mut self, mesh: &mut HalfEdgeMesh) -> Result<bool, MeshError> {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.execute(mesh)?;
            self.undo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Geri alınabilir işlem olup olmadığını döner.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Yinelenebilir işlem olup olmadığını döner.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::create_cube;
    use glam::Vec3;

    #[test]
    fn test_undo_redo_stack() {
        let mut mesh = create_cube(Vec3::ONE);
        let mut history = UndoStack::new(10);

        let initial_vertices = mesh.vertices.len();

        let cmd = SnapshotMeshCommand::new("Extrude Test", |m| {
            m.extrude_face(crate::topology::FaceId(0), 1.0)?;
            Ok(())
        });

        history.execute(cmd, &mut mesh).unwrap();
        assert!(mesh.vertices.len() > initial_vertices);

        // Undo
        history.undo(&mut mesh).unwrap();
        assert_eq!(mesh.vertices.len(), initial_vertices);

        // Redo
        history.redo(&mut mesh).unwrap();
        assert!(mesh.vertices.len() > initial_vertices);
    }
}
