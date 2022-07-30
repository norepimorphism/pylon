// SPDX-License-Identifier: MPL-2.0

//! A small 3D rendering library.
//!
//! # Coordinate Spaces
//!
//! There are four coordinate spaces: **mesh** space, **world** space, **camera** space, and
//! **clip** space.
//!
//! ## Mesh Space
//!
//! Each mesh has an associated mesh space where the origin is considered the 'center' of the mesh.
//! Meshes are scaled and rotated about this origin.
//!
//! Mesh space is unbounded in all axes.
//!
//! ## World Space
//!
//! During object rendering, the coordinates of each object's mesh must be rebased to world space.
//! This is done through matrix addition of the object's world position and each mesh vertex.
//!
//! World space is unbounded in all axes.
//!
//! ## Camera Space
//!
//! This is the fun part: once all objects are in world space, we transform *the world itself* such
//! that the GPU only renders what the camera sees. Here, it is important to make a distinction
//! between the terms 'camera' and 'viewport'; the camera is managed by Pylon and may be positioned
//! arbitrarily in world space, but the viewport is a fixed region produced by compressing the GPU's
//! 3D clip space into a 2D rectangle. As the viewport is immovable, in order to render the scene as
//! seen through Pylon's camera, we must translate, scale, and rotate the vertices of each and every
//! object as dictated by the camera.
//!
//! Camera space is unbounded in all axes.
//!
//! ## Clip Space
//!
//! Clip space is the final destination for vertices and is produced by constraining camera space to
//! the range `[-1, 1]` in all axes. During rasterization, clip space is compressed into a 2D
//! viewport.

#![feature(portable_simd)]

pub mod linear;
pub mod renderer;

pub use linear::{Matrix, Vector};
pub use renderer::Renderer;

/// The integral type for indexing a mesh's [vertex pool](Mesh::vertex_pool).
pub type MeshVertexIndex = u32;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// The location of this camera in world space.
    pub position: Point,
    /// A point, in world space, that this camera is 'looking at'.
    ///
    /// This *target point* determines the pitch and yaw of this camera, so-to-speak. This point
    /// must not be equivalent to [the position](Self::position).
    pub target: Point,
    /// The current roll angle in radians.
    pub roll: f32,
}

impl Camera {
    fn transformation_matrix(&self) -> Matrix {
        // TODO
        Matrix::IDENTITY
    }
}

impl From<Vector> for Point {
    fn from(v: Vector) -> Self {
        let [x, y, z, _] = v.to_array();

        Self { x, y, z }
    }
}

/// A singular location within a coordinate space.
///
/// The `Point` type definition does not prescribe a particular coordinate space to constrain its
/// coordinates by; rather, the coordinates of a `Point` shall be interpreted by context. For
/// example, in the context of clip space, all coordinates within a `Point` must lie between -1 and
/// 1.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// The X coordinate.
    pub x: f32,
    /// The Y coordinate.
    pub y: f32,
    /// The Z coordinate.
    pub z: f32,
}

unsafe impl bytemuck::Pod for Point {}
unsafe impl bytemuck::Zeroable for Point {}

impl Point {
    /// The point that lies at `(0, 0, 0)`.
    pub const ORIGIN: Self = Self { x: 0., y: 0., z: 0. };
}

impl From<Point> for Vector {
    fn from(p: Point) -> Self {
        Self::new(p.x, p.y, p.z, 1.)
    }
}

impl Object {
    /// Creates a new `Object`.
    ///
    /// # Arguments
    ///
    /// - `position`: the position of `mesh` in world space.
    /// - `rotation`: the rotation of `mesh`.
    /// - `scale`: the scale factor of `mesh`. A scale factor of 1 represents the
    ///   original mesh size.
    /// - `material`: The material applied to `mesh`.
    /// - `mesh`: The mesh.
    pub fn new(
        position: Point,
        rotation: Rotation,
        scale: f32,
        material: Material,
        mesh: Mesh,
    ) -> Self {
        Self {
            position,
            rotation,
            scale,
            material,
            mesh,
            resources: None,
        }
    }
}

/// A [mesh](Mesh), with a [material](Material) applied, within a scene.
#[derive(Debug)]
pub struct Object {
    position: Point,
    rotation: Rotation,
    scale: f32,
    material: Material,
    mesh: Mesh,
    resources: Option<ObjectResources>,

}

impl Object {
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

    pub fn material(&self) -> &Material {
        &self.material
    }

    pub fn material_mut(&mut self) -> &mut Material {
        &mut self.material
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    pub fn mutate_mesh(&mut self, f: impl FnOnce(&mut Mesh)) {
        f(&mut self.mesh);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Rotation {
    pub const ZERO: Self = Self { x: 0., y: 0., z: 0. };
}

#[derive(Clone, Debug)]
pub struct Material;

#[derive(Clone, Debug)]
pub struct Mesh {
    /// The vertices that make up this mesh.
    pub vertex_pool: Vec<MeshVertex>,
    /// Triads of vertices from [`Self::vertex_pool`] that define the triangle primitives of this
    /// mesh.
    pub triangles: Vec<MeshTriangle>
}

/// A vertex within a [mesh](Mesh).
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    /// The location of this vertex in mesh space.
    pub point: Point,
}

unsafe impl bytemuck::Pod for MeshVertex {}
unsafe impl bytemuck::Zeroable for MeshVertex {}

impl MeshTriangle {
    /// Creates a new `MeshTriangle` from a triad of vertex indices.
    pub fn new(indices: [MeshVertexIndex; 3]) -> Self {
        Self(indices)
    }
}

/// A triangle within a [mesh](Mesh).
#[derive(Clone, Copy, Debug)]
pub struct MeshTriangle([MeshVertexIndex; 3]);

unsafe impl bytemuck::Pod for MeshTriangle {}
unsafe impl bytemuck::Zeroable for MeshTriangle {}

#[derive(Debug)]
struct ObjectResources {
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl Drop for ObjectResources {
    fn drop(&mut self) {
        self.index_buffer.destroy();
        self.vertex_buffer.destroy();
    }
}
