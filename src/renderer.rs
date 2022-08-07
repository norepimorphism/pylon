// SPDX-License-Identifier: MPL-2.0

use raw_window_handle::HasRawWindowHandle;
use wgpu::*;

use crate::{
    Camera,
    CameraResources,
    MeshVertex,
    Object,
    ObjectResources,
};

/// The hardcoded texture format for [`Renderer::surface`] and which serves as the output of the
/// fragment shader.
const SURFACE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

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
/// [`Renderer::resize_surface`] consumes an argument of this type.
pub struct SurfaceSize {
    /// The width, in pixels, of the surface.
    pub width: u32,
    /// The height, in pixels, of the surface.
    pub height: u32,
}

/// Pylon's 3D renderer.
///
/// From a data perspective, this type is the combination of a surface&mdash;upon which rendering
/// takes place&mdash;and a handle to a GPU. In terms of functionality, a `Renderer` is created with
/// [`new`](Self::new), and [`render`](Self::render) renders a scene to the aforementioned surface.
#[derive(Debug)]
pub struct Renderer {
    device: Device,
    camera_transformation_matrix_bind_group_layout: BindGroupLayout,
    object_transforms_bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
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
        let (
            camera_transformation_matrix_bind_group_layout,
            object_transforms_bind_group_layout,
        ) = Self::create_uniform_bind_group_layouts(&device);
        let pipeline = Self::create_pipeline(
            &device,
            &camera_transformation_matrix_bind_group_layout,
            &object_transforms_bind_group_layout,
        );

        let this = Self {
            device,
            camera_transformation_matrix_bind_group_layout,
            object_transforms_bind_group_layout,
            pipeline,
            queue,
            surface,
        };
        this.resize_surface(surface_size);

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

    fn create_uniform_bind_group_layouts(
        device: &Device,
    ) -> (BindGroupLayout, BindGroupLayout) {
        (
            Self::create_uniform_bind_group_layout(
                device,
                "Pylon camera transformation matrix bind group layout",
            ),
            Self::create_uniform_bind_group_layout(
                device,
                "Pylon object transforms bind group layout",
            ),
        )
    }

    fn create_uniform_bind_group_layout(device: &Device, label: &str) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
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

/// Creates a WGSL shader module from the WGSL code at the given path.
macro_rules! create_wgsl_module {
    ($device:expr, $path:literal $(,)?) => {
        $device.create_shader_module(include_wgsl!($path))
    };
}

impl Renderer {
    fn create_pipeline(
        device: &Device,
        camera_transformation_matrix_bind_group_layout: &BindGroupLayout,
        object_transforms_bind_group_layout: &BindGroupLayout,
    ) -> RenderPipeline {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Pylon pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pylon pipeline layout"),
                bind_group_layouts: &[
                    camera_transformation_matrix_bind_group_layout,
                    object_transforms_bind_group_layout,
                ],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &create_wgsl_module!(device, "shaders/vertex.wgsl"),
                entry_point: "main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: &create_wgsl_module!(device, "shaders/fragment.wgsl"),
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
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
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn create_camera_transformation_matrix_bind_group(
        &self,
        binding: BufferBinding,
    ) -> BindGroup {
        self.create_uniform_bind_group(
            "Pylon camera transformation matrix bind group",
            &self.camera_transformation_matrix_bind_group_layout,
            binding,
        )
    }

    pub fn create_object_transforms_bind_group(&self, binding: BufferBinding) -> BindGroup {
        self.create_uniform_bind_group(
            "Pylon object transforms bind group",
            &self.object_transforms_bind_group_layout,
            binding,
        )
    }

    fn create_uniform_bind_group(
        &self,
        label: &str,
        layout: &BindGroupLayout,
        binding: BufferBinding,
    ) -> BindGroup {
        self.device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(binding),
            }],
        })
    }

    /// Modifies the size of the rendering surface.
    pub fn resize_surface(&self, new_size: SurfaceSize) {
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: SURFACE_FORMAT,
                width: new_size.width,
                height: new_size.height,
                present_mode: PresentMode::AutoNoVsync,
            },
        );
    }

    /// Rasterizes a 3D scene into a 2D frame and sends it to the rendering surface.
    pub fn render<'a, Cr: CameraResources, Or: 'a + ObjectResources>(
        &self,
        camera: &Camera<Cr>,
        objects: impl IntoIterator<Item = &'a Object<Or>>,
    ) {
        let frame = self.surface.get_current_texture().unwrap();
        let frame_view = Self::create_frame_view(&frame.texture);
        let mut encoder = self.create_command_encoder();

        {
            let mut pass = Self::create_render_pass(&mut encoder, &frame_view);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(
                0,
                camera.resources.transformation_matrix_bind_group(),
                &[],
            );

            for object in objects {
                tracing::debug!("Rendering {} triangles...", object.mesh.triangles.len());

                pass.set_bind_group(
                    1,
                    object.resources.transforms_bind_group(),
                    &[],
                );
                pass.set_vertex_buffer(
                    0,
                    object.resources.vertex_buffer(),
                );
                pass.set_index_buffer(
                    object.resources.index_buffer(),
                    IndexFormat::Uint32,
                );

                let index_count = (3 * object.mesh.triangles.len()) as u32;
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));

        frame.present();
    }

    /// Creates a texture view for the current surface frame.
    fn create_frame_view(frame: &Texture) -> TextureView {
        frame.create_view(&TextureViewDescriptor {
            label: Some("Pylon frame view"),
            // I think we can leave most of these as the defaults and *wgpu* will fill them in for
            // us.
            format: None,
            dimension: None,
            aspect: TextureAspect::All,
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
            depth_stencil_attachment: None,
        })
    }
}
