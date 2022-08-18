// SPDX-License-Identifier: MPL-2.0

//! A small 3D rendering library.
//!
//! # Memory Management
//!
//! Pylon cannot predict the future&mdash;*yet*&mdash;so it assumes that library consumers know best
//! about how their objects are created and used and, more importantly, how memory is optimally
//! managed. As such, memory management is performed externally and interfaced through Pylon via
//! [`Camera`] and [`Object`].
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
pub mod tree;

pub use linear::{Matrix, Vector};
pub use renderer::Renderer;

/// The integral type for indexing a mesh's vertex pool.
pub type MeshVertexIndex = u32;

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
/// 1. It should also be noted that the fields [`x`](Self::x), [`y`](Self::y), and [`z`](Self::z)
/// are unlimited and may contain arbitrary values.
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

impl Point {
    /// The point that lies at `(0, 0, 0)`.
    pub const ORIGIN: Self = Self { x: 0., y: 0., z: 0. };
}

impl From<Point> for Vector {
    fn from(p: Point) -> Self {
        Self::new(p.x, p.y, p.z, 1.)
    }
}

/// Gimbal rotation across three axes.
///
/// [`x`](Self::x), [`y`](Self::y), and [`z`](Self::z) are in radians. The Z axis is rotated first,
/// followed by Y and then X.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rotation {
    /// The rotation, in radians, in the X axis.
    ///
    /// During transformation matrix generation, this rotation is applied third.
    pub x: f32,
    /// The rotation, in radians, in the Y axis.
    ///
    /// During transformation matrix generation, this rotation is applied second.
    pub y: f32,
    /// The rotation, in radians, in the Z axis.
    ///
    /// During transformation matrix generation, this rotation is applied first.
    pub z: f32,
}

impl Rotation {
    pub const ZERO: Self = Self { x: 0., y: 0., z: 0. };
}

/// A vertex within a mesh.
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    /// The location of this vertex in mesh space.
    pub point: Point,
}

unsafe impl bytemuck::Pod for MeshVertex {}
unsafe impl bytemuck::Zeroable for MeshVertex {}

impl MeshTriangle {
    /// Creates a new `MeshTriangle` from a triad of vertex indices.
    pub const fn new(indices: [MeshVertexIndex; 3]) -> Self {
        Self(indices)
    }
}

/// A triangle within a mesh.
#[derive(Clone, Copy, Debug)]
pub struct MeshTriangle([MeshVertexIndex; 3]);

unsafe impl bytemuck::Pod for MeshTriangle {}
unsafe impl bytemuck::Zeroable for MeshTriangle {}

/// The interface to user-managed camera resources.
pub trait Camera {
    fn transforms_uniform(&self) -> &CameraTransformsUniform;
}

/// The interface to user-managed object resources.
pub trait Object {
    fn triangle_count(&self) -> u32;

    /// The [pipeline](wgpu::RenderPipeline) to be used during rendering of this object.
    ///
    /// This type may be created via [`Renderer::create_pipeline`].
    fn render_pipeline(&self) -> &wgpu::RenderPipeline;

    fn transforms_uniform(&self) -> &ObjectTransformsUniform;

    /// The [bind group slots](BindGroupSlot) to be assigned for rendering this object.
    ///
    /// Slots are assigned in ascending order of index. If a slot is assigned to twice, the first
    /// assignment is overwritten.
    fn bind_group_slots<'a>(&'a self) -> &[BindGroupSlot<'a>];

    /// A slice into a GPU buffer that contains the vertex index sequences that form the triangle
    /// mesh of this object.
    ///
    /// To guarantee vertex shader compatibility, this buffer should contain a packed sequence of
    /// [`MeshTriangle`]s.
    fn index_buffer<'a>(&'a self) -> wgpu::BufferSlice<'a>;

    /// A slice into a GPU buffer that contains the vertex data for this object.
    ///
    /// To guarantee vertex shader compatibility, this buffer should contain a sequence of
    /// [`MeshVertex`]s.
    fn vertex_buffer<'a>(&'a self) -> wgpu::BufferSlice<'a>;
}

pub struct CameraTransformsUniform(TransformsUniform);

pub struct ObjectTransformsUniform(TransformsUniform);

struct TransformsUniform {
    bind_group: wgpu::BindGroup,
}

/// The assignment of [a bind group](wgpu::BindGroup) to a bind group slot.
pub struct BindGroupSlot<'a> {
    /// The index of the slot that [the bind group](Self::bind_group) should inhabit.
    pub index: u32,
    /// The bind group to be assigned to the slot described by [`index`](Self::index).
    pub bind_group: &'a wgpu::BindGroup,
}
