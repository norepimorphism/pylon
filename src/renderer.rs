// SPDX-License-Identifier: MPL-2.0

use raw_window_handle::HasRawWindowHandle;
use wgpu::{*, util::DeviceExt as _};

use crate::{MeshVertex, Object, Point, Scene};

const SURFACE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

#[derive(Debug)]
pub enum Error {
    NoCompatibleAdapterFound,
    NoCompatibleDeviceFound,
}

/// Pylon's 3D renderer.
#[derive(Debug)]
pub struct Renderer {
    device: Device,
    pipeline: RenderPipeline,
    queue: Queue,
    surface: Surface,
    uniforms: Uniforms,
}

#[derive(Debug)]
struct Uniforms {
    object_transforms: Uniform,
    camera: Uniform,
}

#[derive(Debug)]
struct Uniform {
    buffer: Buffer,
    bind_group: BindGroup,
    bind_group_layout: BindGroupLayout,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct ObjectTransforms {
    position: Point,
    scale: f32,
}

unsafe impl bytemuck::Pod for ObjectTransforms {}
unsafe impl bytemuck::Zeroable for ObjectTransforms {}

impl Renderer {
    /// Creates a new `Renderer`.
    ///
    /// # Safety
    ///
    /// `window` must live for as long as the returned renderer.
    pub async unsafe fn new(
        window: &impl HasRawWindowHandle,
        backends: Backends,
        surface_width: u32,
        surface_height: u32,
    ) -> Result<Self, Error> {
        let (adapter, surface) = Self::create_adapter_and_surface(window, backends).await?;
        let surface_formats = surface.get_supported_formats(&adapter);
        if !surface_formats.contains(&SURFACE_FORMAT) {
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
        let uniforms = Self::create_uniforms(&device);
        let pipeline = Self::create_pipeline(&device, &uniforms);

        let mut this = Self {
            device,
            pipeline,
            queue,
            surface,
            uniforms,
        };
        this.resize_surface(surface_width, surface_height);

        Ok(this)
    }

    /// Creates handles to the graphics backend as well as the surface upon which rendering will
    /// take place.
    async fn create_adapter_and_surface(
        window: &impl HasRawWindowHandle,
        backends: Backends,
    ) -> Result<(Adapter, Surface), Error> {
        let instance = Instance::new(backends);

        // SAFETY: [`Instance::create_surface`] requires that the window is valid and will live for
        // the lifetime of the returned surface. It would be a bug in `minifb` for the first
        // invariant not to hold, and the second holds because both the window and surface live
        // until the end of [`Ui::run`].
        let surface = unsafe { instance.create_surface(window) };

        instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface),
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
                features: Features::POLYGON_MODE_LINE,
                ..Default::default()
            },
            None,
        )
        .await
        .map_err(|_| Error::NoCompatibleDeviceFound)
    }

    fn create_uniforms(device: &Device) -> Uniforms {
        Uniforms {
            object_transforms: Self::create_uniform(
                device,
                std::mem::size_of::<ObjectTransforms>(),
            ),
            camera: Self::create_uniform(
                device,
                std::mem::size_of::<[[f32; 4]; 4]>(),
            ),
        }
    }

    fn create_uniform(device: &Device, size: usize) -> Uniform {
        let buffer = Self::create_uniform_buffer(device, size);
        let bind_group_layout = Self::create_uniform_bind_group_layout(device);

        Uniform {
            bind_group: Self::create_uniform_bind_group(
                device,
                &bind_group_layout,
                &buffer,
            ),
            buffer,
            bind_group_layout,
        }
    }

    fn create_uniform_buffer(device: &Device, size: usize) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: None,
            size: size as BufferAddress,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        })
    }

    fn create_uniform_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
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

    fn create_uniform_bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(buffer.as_entire_buffer_binding()),
            }],
        })
    }
}

macro_rules! create_shader_module {
    ($device:expr, $path:literal $(,)?) => {
        $device.create_shader_module(include_wgsl!($path))
    };
}

impl Renderer {
    fn create_pipeline(
        device: &Device,
        uniforms: &Uniforms,
    ) -> RenderPipeline {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &uniforms.camera.bind_group_layout,
                    &uniforms.object_transforms.bind_group_layout,
                ],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &create_shader_module!(device, "shaders/vertex.wgsl"),
                entry_point: "main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: &create_shader_module!(device, "shaders/fragment.wgsl"),
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: SURFACE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                polygon_mode: PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: SURFACE_FORMAT,
                width,
                height,
                present_mode: PresentMode::Fifo,
            },
        );
    }

    pub fn render(&mut self, scene: &Scene) {
        tracing::info!("Rendering {} object(s)...", scene.objects.len());

        let frame = self.surface.get_current_texture().unwrap();
        let frame_view = Self::create_texture_view(&frame.texture);
        let object_resources: Vec<ObjectResources> = scene
            .objects
            .iter()
            .map(|object| self.create_object_resources(object))
            .collect();
        self.queue.write_buffer(
            &self.uniforms.camera.buffer,
            0,
            bytemuck::bytes_of(&scene.camera.transformation_matrix().to_array()),
        );

        let mut encoder = self.create_command_encoder();
        {
            let mut pass = Self::create_render_pass(&mut encoder, &frame_view);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniforms.camera.bind_group, &[]);
            for (i, object) in scene.objects.iter().enumerate() {
                tracing::debug!("Rendering {} triangles...", object.mesh.triangles.len());

                self.queue.write_buffer(
                    &self.uniforms.object_transforms.buffer,
                    0,
                    bytemuck::bytes_of(&self.create_object_transforms(object)),
                );

                let object_resources = &object_resources[i];
                pass.set_bind_group(
                    1,
                    &self.uniforms.object_transforms.bind_group,
                    &[],
                );
                pass.set_vertex_buffer(
                    0,
                    object_resources.vertex_buffer.slice(..),
                );
                pass.set_index_buffer(
                    object_resources.index_buffer.slice(..),
                    IndexFormat::Uint32,
                );

                let index_count = (3 * object.mesh.triangles.len()) as u32;
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));

        frame.present();
    }

    fn create_texture_view(texture: &Texture) -> TextureView {
        texture.create_view(&TextureViewDescriptor {
            label: None,
            format: None,
            dimension: None,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        })
    }
}

struct ObjectResources {
    index_buffer: Buffer,
    vertex_buffer: Buffer,
}

impl Renderer {
    fn create_object_resources(&self, object: &Object) -> ObjectResources {
        ObjectResources {
            index_buffer: self.create_buffer(
                &object.mesh.triangles,
                BufferUsages::INDEX,
            ),
            vertex_buffer: self.create_buffer(
                &object.mesh.vertex_pool,
                BufferUsages::VERTEX,
            ),
        }
    }

    fn create_buffer<T>(&self, slice: &[T], usage: BufferUsages) -> Buffer
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(slice),
            usage,
        })
    }

    fn create_command_encoder(&self) -> CommandEncoder {
        self.device.create_command_encoder(&CommandEncoderDescriptor::default())
    }

    fn create_render_pass<'a>(
        encoder: &'a mut CommandEncoder,
        view: &'a TextureView,
    ) -> RenderPass<'a> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::WHITE),
                    store: true,
                },
            })],
            ..Default::default()
        })
    }

    fn create_object_transforms(&self, object: &Object) -> ObjectTransforms {
        ObjectTransforms {
            position: object.position,
            scale: object.scale,
        }
    }
}
