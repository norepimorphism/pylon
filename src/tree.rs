use std::{cell::Cell, rc::Weak};

use crate::{Matrix, Point, Rotation};

pub struct Node {
    parent: Weak<Node>,
    /// The position of this node relative to its parent.
    position: Point,
    /// The rotation of this node relative to the rotation of its parent.
    rotation: Rotation,
    /// Cached global and local transformation matrices.
    ///
    /// If a transformation matrix is available and valid from a previous call to
    /// [`global_transformation_matrix`](Self::global_transformation_matrix) or
    /// [`local_transformation_matrix`](Self::local_transformation_matrix), it is pulled from here.
    /// Otherwise, the newly-created matrix is cached to here.
    cached_transformation_matrices: CachedTransformationMatrices,
}

impl Node {
    pub fn local_transformation_matrix(&self) -> Matrix {
        if let Some(matrix) = self.cached_transformation_matrices.local.get() {
            return matrix;
        }

        let matrix = self.create_local_transformation_matrix();

        todo!()
    }

    fn create_local_transformation_matrix(&self) -> Matrix {
        todo!()
    }
}

#[derive(Debug)]
struct CachedTransformationMatrices {
    global: Cell<Option<Matrix>>,
    local: Cell<Option<Matrix>>,
}
