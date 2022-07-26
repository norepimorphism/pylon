// SPDX-License-Identifier: MPL-2.0

//! Pylon's 3D renderer.

use raw_window_handle::HasRawWindowHandle;
use wgpu::*;

use crate::{
    CameraTransformsUniform,
    MeshVertex,
    ObjectTransformsUniform,
    TransformsUniform,
};
pub use render::Job;

mod render;

/// The hardcoded texture format for [`Renderer::surface`] and which serves as the output of the
/// fragment shader.
const SURFACE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24Plus;

/// The cause of a failure during [`Renderer` creation](Renderer::new).
#[derive(Debug)]
pub enum Error {
    /// A graphics adapter was requested but none was returned.
    ///
    /// This could be for a few reasons:
    /// 1. instance creation failed due to unavailable backends;
    /// 2. the rendering surface produced from the given window was invalid;
    /// 3. the given power preference did not match any available graphics adapters; or
    /// 4. *wgpu*, your OS, or your graphics drivers failed.
    NoCompatibleAdapterFound,
    /// A handle to a graphics device was requested but none was returned.
    ///
    /// This error is likely rare and may represent a problem outside the control of Pylon.
    NoCompatibleDeviceFound,
}

/// The physical dimensions of a rendering surface.
///
/// [`Renderer::configure_surface`] consumes an argument of this type.
pub struct SurfaceSize {
    /// The width, in pixels, of the surface.
    pub width: u32,
    /// The height, in pixels, of the surface.
    pub height: u32,
}

/// Layouts of Pylon's built-in bind groups.
///
/// A [renderer](Renderer) creates this once and references it during pipeline creation.
#[derive(Debug)]
struct BuiltinBindGroupLayouts {
    /// The layout of the camera transformation matrix bind group.
    for_camera: BindGroupLayout,
    /// The layout of the object transformation matrix bind group.
    for_object: BindGroupLayout,
}

impl BuiltinBindGroupLayouts {
    /// Creates a new `BuiltinBindGroupLayouts`.
    fn new(device: &Device) -> Self {
        Self {
            for_camera: Self::create_layout(
                device,
                "Pylon camera transformation matrix bind group layout",
            ),
            for_object: Self::create_layout(
                device,
                "Pylon object transformation matrix bind group layout",
            ),
        }
    }

    /// Creates the layout of a built-in bind group.
    ///
    /// As it happens that Pylon's built-in bind groups are identical in all but name, the `label`
    /// field governs which layout this function produces.
    fn create_layout(
        device: &Device,
        label: &str,
    ) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[BindGroupLayoutEntry {
                // This must match the binding in the vertex shader.
                binding: 0,
                // This layout need only be visible in the vertex shader. The fragment shader is
                // completely user-controlled.
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
}

/// Pylon's 3D renderer.
///
/// From a data perspective, this type is the combination of a surface&mdash;upon which rendering
/// takes place&mdash;and a handle to a GPU. In terms of functionality, a `Renderer` is created with
/// [`new`](Self::new), and [`render`](Self::render) renders a scene to the aforementioned surface.
#[derive(Debug)]
pub struct Renderer {
    /// Layouts of Pylon's built-in bind groups.
    ///
    /// This field is populated once during [`new`](Self::new) and should be considered immutable
    /// afterwards.
    builtin_bind_group_layouts: BuiltinBindGroupLayouts,
    depth: Texture,
    device: Device,
    queue: Queue,
    surface: Surface,
}

impl Renderer {
    /// Creates a new `Renderer`.
    ///
    /// # Safety
    ///
    /// `window` must be valid and must live for as long as the returned renderer.
    pub async unsafe fn new(
        window: &impl HasRawWindowHandle,
        backends: Backends,
        adapter_power_pref: PowerPreference,
        surface_size: SurfaceSize,
        present_mode: PresentMode,
    ) -> Result<Self, Error> {
        let (adapter, surface) = Self::create_adapter_and_surface(
            window,
            backends,
            adapter_power_pref,
        )
        .await?;

        let surface_formats = surface.get_supported_formats(&adapter);
        // Pipeline creation will probably panic later if the hardcoded surface format is
        // unsupported.
        if !surface_formats.contains(&SURFACE_FORMAT) {
            // TODO: We should support a few other formats to fall-back on.
            todo!(
                "Unsupported surface format; available are: {}",
                surface_formats
                    .iter()
                    .map(|format| format!("{:?}", format))
                    .collect::<Vec<String>>()
                    .join(", "),
            );
        }

        let (device, queue) = Self::create_device_and_queue(&adapter).await?;
        let builtin_bind_group_layouts = BuiltinBindGroupLayouts::new(&device);
        let depth = Self::create_depth(
            &device,
            surface_size.width,
            surface_size.height,
        );

        let mut this = Self {
            builtin_bind_group_layouts,
            depth,
            device,
            queue,
            surface,
        };
        // The surface must be configured before it is usable.
        this.configure_surface(surface_size, present_mode);

        Ok(this)
    }

    /// Creates handles to the graphics backend as well as the surface upon which rendering will
    /// take place.
    async fn create_adapter_and_surface(
        window: &impl HasRawWindowHandle,
        backends: Backends,
        adapter_power_pref: PowerPreference,
    ) -> Result<(Adapter, Surface), Error> {
        let instance = Instance::new(backends);

        // SAFETY: [`Renderer::new`]'s safety contract promises that `window` is valid and will live
        // for as long as `surface`.
        let surface = unsafe { instance.create_surface(window) };

        instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: adapter_power_pref,
            ..Default::default()
        })
        .await
        .ok_or_else(|| Error::NoCompatibleAdapterFound)
        .map(|adapter| (adapter, surface))
    }

    /// Creates handles to the logical graphics device as well as the command buffer queue.
    async fn create_device_and_queue(adapter: &Adapter) -> Result<(Device, Queue), Error> {
        adapter.request_device(
            &DeviceDescriptor {
                limits: adapter.limits(),
                ..Default::default()
            },
            None,
        )
        .await
        .map_err(|_| Error::NoCompatibleDeviceFound)
    }

    fn create_depth(device: &Device, width: u32, height: u32) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("Pylon depth texture"),
            size: Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
        })
    }

    /// Configures the rendering surface.
    ///
    /// This is automatically called during [`new`](Self::new). It may be called again to resize the
    /// surface or modify the presentation mode.
    pub fn configure_surface(&mut self, size: SurfaceSize, present_mode: PresentMode) {
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: SURFACE_FORMAT,
                width: size.width,
                height: size.height,
                present_mode,
            },
        );
        self.depth = Self::create_depth(&self.device, size.width, size.height);
    }
}

/// Creates a WGSL shader module from the WGSL code at the given path.
macro_rules! create_wgsl_module_from_path {
    ($device:expr, $path:literal $(,)?) => {
        $device.create_shader_module(include_wgsl!($path))
    };
}

impl Renderer {
    /// Creates a render pipeline for [an object](Object).
    pub fn create_pipeline(
        &self,
        fragment_shader: &ShaderModule,
    ) -> RenderPipeline {
        self.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Pylon pipeline"),
            layout: Some(&self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pylon pipeline layout"),
                bind_group_layouts: &[
                    &self.builtin_bind_group_layouts.for_camera,
                    &self.builtin_bind_group_layouts.for_object,
                ],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &create_wgsl_module_from_path!(self.device, "shaders/vertex.wgsl"),
                entry_point: "main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: fragment_shader,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    // The output of the fragment shader must be compatible with this format.
                    format: SURFACE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                polygon_mode: PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Creates a new `CameraTransformsUniform` with the given buffer binding.
    ///
    /// If the backing storage for the returned uniform changes, it *must* be recreated by calling
    /// this function again with the new buffer binding.
    pub fn create_camera_transforms_uniform(
        &self,
        binding: BufferBinding,
    ) -> CameraTransformsUniform {
        CameraTransformsUniform(
            self.create_transforms_uniform(
                "Pylon camera transformation matrix bind group",
                &self.builtin_bind_group_layouts.for_camera,
                binding,
            )
        )
    }

    /// Creates a new `ObjectTransformsUniform` with the given buffer binding.
    ///
    /// If the backing storage for the returned uniform changes, it *must* be recreated by calling
    /// this function again with the new buffer binding.
    pub fn create_object_transforms_uniform(
        &self,
        binding: BufferBinding,
    ) -> ObjectTransformsUniform {
        ObjectTransformsUniform(
            self.create_transforms_uniform(
                "Pylon object transforms bind group",
                &self.builtin_bind_group_layouts.for_object,
                binding,
            )
        )
    }

    /// Creates a new `TransformsUniform`.
    ///
    /// As it happens that Pylon's built-in bind groups are identical in all but name, the
    /// `bind_group_label` field governs which bind group this function produces.
    fn create_transforms_uniform(
        &self,
        bind_group_label: &str,
        bind_group_layout: &BindGroupLayout,
        binding: BufferBinding,
    ) -> TransformsUniform {
        TransformsUniform {
            bind_group: self.device.create_bind_group(&BindGroupDescriptor {
                label: Some(bind_group_label),
                layout: bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(binding),
                }],
            }),
        }
    }

    pub fn create_render<'a>(&'a self) -> Job<'a> {
        Job::new(&self.surface, &self.depth, &self.device, &self.queue)
    }
}
