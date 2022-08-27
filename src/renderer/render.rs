impl<'a> Job<'a> {
    pub(super) fn new(
        surface: &wgpu::Surface,
        depth: &wgpu::Texture,
        device: &wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> Self {
        let frame = surface.get_current_texture().unwrap();

        Job {
            frame_view: Self::create_frame_view(&frame.texture),
            frame,
            depth_view: Self::create_depth_view(depth),
            encoder: Self::create_command_encoder(device),
            queue: &queue,
        }
    }

    /// Creates a texture view for the current surface frame.
    fn create_frame_view(frame: &wgpu::Texture) -> wgpu::TextureView {
        Self::create_texture_view(
            frame,
            "Pylon frame view",
            wgpu::TextureAspect::All,
        )
    }

    fn create_depth_view(depth: &wgpu::Texture) -> wgpu::TextureView {
        Self::create_texture_view(
            depth,
            "Pylon depth view",
            wgpu::TextureAspect::DepthOnly,
        )
    }

    fn create_texture_view(
        texture: &wgpu::Texture,
        label: &str,
        aspect: wgpu::TextureAspect,
    ) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
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

    fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pylon command encoder")
        })
    }
}

pub struct Job<'a> {
    frame: wgpu::SurfaceTexture,
    frame_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
    queue: &'a wgpu::Queue,
}

impl Job<'_> {
    pub fn add_pass<'this>(&'this mut self, camera: CameraTransformsUniform) -> Pass<'this> {
        Pass(self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pylon surface frame render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // We can either clear or load here. Clearing wipes the frame with a given color
                    // while loading initializes the frame with the current state of the surface.
                    load: wgpu::LoadOp::Load,
                    // The surface frame contains the final result of the render, so obviously we
                    // need to write to it.
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    // In clip space, 1.0 is the maximmum depth.
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        }))
    }

    pub fn submit(self) {
        self.queue.submit(Some(self.encoder.finish()));
        self.frame.present();
    }
}

pub struct Pass<'a>(wgpu::RenderPass<'a>);

impl Pass<'_> {
    pub fn with_camera(self, camera: CameraTransformsUniform) {
        self.0.set_bind_group(
            0,
            &camera.0.bind_group,
            &[],
        );

        self
    }

    pub fn draw_object<'a>(
        &self,
        pipeline: &wgpu::RenderPipeline,
        bind_group_slots: &[BindGroupSlot<'a>],
        transforms_uniform: &ObjectTransformsUniform,
        vertex_buffer: wgpu::BufferSlice,
        index_buffer: wgpu::BufferSlice,
    ) {
        let triangle_count = object.triangle_count();

        tracing::debug!("Rendering {} triangles...", triangle_count);

        self.0.set_pipeline(pipeline);
        self.0.set_bind_group(
            1,
            &object.transforms_uniform().0.bind_group,
            &[],
        );
        for slot in bind_group_slots {
            if slot.index < 2 {
                panic!("slots 0 and 1 cannot be overwritten");
            }

            self.0.set_bind_group(
                slot.index,
                slot.bind_group,
                &[],
            );
        }
        self.0.set_vertex_buffer(0, vertex_buffer);
        self.0.set_index_buffer(index_buffer, wgpu::IndexFormat::Uint32);

        let index_count = (3 * triangle_count) as u32;
        self.0.draw_indexed(0..index_count, 0, 0..1);
    }
}
