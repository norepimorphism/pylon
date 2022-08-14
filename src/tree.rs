use std::{cell::Cell, rc::Weak};

use crate::{Matrix, Point, Rotation, Vector};

impl Default for Node {
    fn default() -> Self {
        Self {
            parent: Weak::new(),
            position: Point::ORIGIN,
            rotation: Rotation::ZERO,
            scale: 1.0,
            cached_transformation_matrices: Default::default(),
        }
    }
}

pub struct Node {
    parent: Weak<Node>,
    /// The position of this node relative to its parent.
    position: Point,
    /// The rotation of this node relative to the rotation of its parent.
    rotation: Rotation,
    /// The scale factor of this node's coordinates.
    scale: f32,
    /// Cached global and local transformation matrices.
    ///
    /// If a transformation matrix is available and valid from a previous call to
    /// [`global_transformation_matrix`](Self::global_transformation_matrix) or
    /// [`local_transformation_matrix`](Self::local_transformation_matrix), it is pulled from here.
    /// Otherwise, the newly-created matrix is cached to here.
    cached_transformation_matrices: CachedTransformationMatrices,
}

impl Node {
    pub fn parent(&self) -> &Weak<Node> {
        &self.parent
    }

    pub fn parent_mut(&mut self) -> &mut Weak<Node> {
        &mut self.parent
    }

    pub fn position(&self) -> Point {
        self.position
    }

    pub fn position_mut(&mut self) -> &mut Point {
        &mut self.position
    }

    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    pub fn rotation_mut(&mut self) -> &mut Rotation {
        &mut self.rotation
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn scale_mut(&mut self) -> &mut f32 {
        &mut self.scale
    }

    pub fn invalidate_global_cache(&self) {
        self.cached_transformation_matrices.invalidate_global();
    }

    pub fn invalidate_cache(&self) {
        self.cached_transformation_matrices.invalidate_all();
    }

    /// The global transformation matrix for this node.
    ///
    /// This will return a cached copy if one is available.
    pub fn global_transformation_matrix(&self) -> Matrix {
        self.transformation_matrix(
            &self.cached_transformation_matrices.global,
            Self::create_global_transformation_matrix,
        )
    }

    /// The local transformation matrix for this node.
    ///
    /// This will return a cached copy if one is available.
    pub fn local_transformation_matrix(&self) -> Matrix {
        self.transformation_matrix(
            &self.cached_transformation_matrices.local,
            Self::create_local_transformation_matrix,
        )
    }

    fn transformation_matrix(
        &self,
        cell: &Cell<Option<Matrix>>,
        create: impl FnOnce(&Self) -> Matrix,
    ) -> Matrix {
        if let Some(matrix) = cell.get() {
            return matrix;
        }

        let matrix = create(self);
        cell.set(Some(matrix));

        matrix
    }
}

impl Default for CachedTransformationMatrices {
    fn default() -> Self {
        Self {
            global: Cell::new(None),
            local: Cell::new(None),
        }
    }
}

#[derive(Debug)]
struct CachedTransformationMatrices {
    global: Cell<Option<Matrix>>,
    local: Cell<Option<Matrix>>,
}

impl CachedTransformationMatrices {
    fn invalidate_global(&self) {
        self.global.set(None);
    }

    fn invalidate_all(&self) {
        self.invalidate_global();
        self.local.set(None);
    }
}

impl Node {
    fn create_global_transformation_matrix(&self) -> Matrix {
        let mut matrix = self.local_transformation_matrix();

        // Because we're using pre-multiplication, the order of application is in reverse;
        // although the local transformation matrix is applied last, we start with the local
        // transformation matrix and traverse the tree upwards.
        if let Some(node) = self.parent.upgrade() {
            matrix *= node.global_transformation_matrix();
        }

        matrix
    }

    /// Creates a local transformation matrix for this node.
    ///
    /// This is the product of local position, rotation, and scale matrices.
    fn create_local_transformation_matrix(&self) -> Matrix {
        // Because we're using pre-multiplication, the order here is reversed. The true order is:
        // 1. Scale.
        // 2. Rotate.
        // 3. Translate.
        return
            self.create_local_position_matrix() *
            self.create_local_rotation_matrix() *
            self.create_local_scale_matrix();
    }

    /// Creates a local transformation matrix for the position transform of this node.
    ///
    /// This transform is applied third.
    fn create_local_position_matrix(&self) -> Matrix {
        let mut m = Matrix::IDENTITY;
        m.columns_mut()[3] += Vector::from(self.position);

        return m;
    }

    /// Creates a local transformation matrix for the rotation transform of this node.
    ///
    /// This transform is applied third.
    fn create_local_rotation_matrix(&self) -> Matrix {
        return
            self.create_local_x_rotation_matrix() *
            self.create_local_y_rotation_matrix() *
            self.create_local_z_rotation_matrix();
    }

    /// Creates a local transformation matrix for the X rotation transform of this node.
    fn create_local_x_rotation_matrix(&self) -> Matrix {
        let SinCos { sin: s, cos: c } = SinCos::new(self.rotation.x);

        Matrix::new(
            1., 0., 0., 0.,
            0.,  c, -s, 0.,
            0.,  s,  c, 0.,
            0., 0., 0., 1.,
        )
    }

    /// Creates a local transformation matrix for the Y rotation transform of this node.
    fn create_local_y_rotation_matrix(&self) -> Matrix {
        let SinCos { sin: s, cos: c } =  SinCos::new(self.rotation.y);

        Matrix::new(
             c, 0.,  s, 0.,
            0., 1., 0., 0.,
            -s, 0.,  c, 0.,
            0., 0., 0., 1.,
        )
    }

    /// Creates a local transformation matrix for the Z rotation transform of this node.
    fn create_local_z_rotation_matrix(&self) -> Matrix {
        let SinCos { sin: s, cos: c } = SinCos::new(self.rotation.z);

        Matrix::new(
             c, -s, 0., 0.,
             s,  c, 0., 0.,
            0., 0., 1., 0.,
            0., 0., 0., 1.,
        )
    }

    /// Creates a local transformation matrix for scale transform of this node.
    ///
    /// This transform is applied first.
    fn create_local_scale_matrix(&self) -> Matrix {
        let f = self.scale;

        Matrix::new(
             f, 0., 0., 0.,
            0.,  f, 0., 0.,
            0., 0.,  f, 0.,
            0., 0., 0., 1.,
        )
    }
}

impl SinCos {
    fn new(radians: f32) -> Self {
        Self {
            sin: radians.sin(),
            cos: radians.cos(),
        }
    }
}

struct SinCos {
    sin: f32,
    cos: f32,
}
