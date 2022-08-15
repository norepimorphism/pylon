// SPDX-License-Identifier: MPL-2.0

//! Pylon's 3D renderer.

use raw_window_handle::HasRawWindowHandle;
use wgpu::*;

use crate::{
    Camera,
    CameraTransformsUniform,
    MeshVertex,
    Object,
    ObjectTransformsUniform,
    TransformsUniform,
};

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
        fragment_shader: ShaderSource,
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
                module: &self.device.create_shader_module(ShaderModuleDescriptor {
                    label: Some("Pylon fragment shader"),
                    source: fragment_shader,
                }),
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

    /// Rasterizes a 3D scene into a 2D frame and sends it to the rendering surface.
    pub fn render<'a, C: Camera, O: 'a + Object>(
        &self,
        camera: &C,
        objects: impl IntoIterator<Item = &'a O>,
    ) {
        let frame = self.surface.get_current_texture().unwrap();
        let frame_view = Self::create_frame_view(&frame.texture);
        let depth_view = Self::create_depth_view(&self.depth);
        let mut encoder = self.create_command_encoder();

        {
            let mut pass = Self::create_render_pass(
                &mut encoder,
                &frame_view,
                &depth_view,
            );
            pass.set_bind_group(
                0,
                &camera.transforms_uniform().0.bind_group,
                &[],
            );

            for object in objects {
                let triangle_count = object.triangle_count();

                tracing::debug!("Rendering {} triangles...", triangle_count);

                pass.set_pipeline(object.render_pipeline());
                pass.set_bind_group(
                    1,
                    &object.transforms_uniform().0.bind_group,
                    &[],
                );
                for slot in object.bind_group_slots() {
                    if slot.index < 2 {
                        panic!("slots 0 and 1 cannot be overwritten");
                    }

                    pass.set_bind_group(
                        slot.index,
                        slot.bind_group,
                        &[],
                    );
                }
                pass.set_vertex_buffer(
                    0,
                    object.vertex_buffer(),
                );
                pass.set_index_buffer(
                    object.index_buffer(),
                    IndexFormat::Uint32,
                );

                let index_count = (3 * triangle_count) as u32;
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));

        frame.present();
    }

    /// Creates a texture view for the current surface frame.
    fn create_frame_view(frame: &Texture) -> TextureView {
        Self::create_texture_view(
            frame,
            "Pylon frame view",
            TextureAspect::All,
        )
    }

    fn create_depth_view(depth: &Texture) -> TextureView {
        Self::create_texture_view(
            depth,
            "Pylon depth view",
            TextureAspect::DepthOnly,
        )
    }

    fn create_texture_view(
        texture: &Texture,
        label: &str,
        aspect: TextureAspect,
    ) -> TextureView {
        texture.create_view(&TextureViewDescriptor {
            label: Some(label),
            // I think we can leave most of these as the defaults and *wgpu* will fill them in for
            // us.
            format: None,
            dimension: None,
            aspect,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        })
    }

    /// Creates a new command encoder for use in [`render`](Self::render).
    fn create_command_encoder(&self) -> CommandEncoder {
        self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Pylon command encoder")
        })
    }

    /// Creates the render pass for the current surface frame.
    fn create_render_pass<'a>(
        encoder: &'a mut CommandEncoder,
        frame_view: &'a TextureView,
        depth_view: &'a TextureView,
    ) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Pylon surface frame render pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: Operations {
                    // We can either clear or load here. Clearing wipes the frame with a given color
                    // while loading initializes the frame with the current state of the surface.
                    load: LoadOp::Load,
                    // The surface frame contains the final result of the render, so obviously we
                    // need to write to it.
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        })
    }
}
