use std::{mem, ops::Range};

use fps_counter::FPSCounter;
use pylon_engine::{
    BindGroupSlot,
    CameraTransformsUniform,
    Material,
    Matrix,
    MeshTriangle,
    MeshVertex,
    ObjectTransformsUniform,
    Point,
    Renderer,
};
use wgpu::BufferAddress;
use wgpu_allocators::{Allocator as _, HeapUsages, NonZeroBufferAddress};
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

/// The width and height, in pixels, of the window that will be rendered to.
const WINDOW_LENGTH: u32 = 512;

/// Runs the cube demo.
fn main() {
    init_tracing();
    let mut fps_counter = FPSCounter::new();
    let event_loop = EventLoop::new();
    let window = create_window(&event_loop);

    let gfx = create_gfx(&window);
    let mut command_encoder = gfx.device().create_command_encoder(
        &wgpu::CommandEncoderDescriptor { label: None },
    );
    // We will store the camera and object transformation matrices in this heap.
    let uniform_heap = wgpu_allocators::Heap::new(
        gfx.device(),
        // To make things simple, we'll allocate 512 bytes, which should be more than enough to
        // store both transformation matrices.
        // SAFETY: 512 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(512) },
        HeapUsages::UNIFORM,
    );
    let mut uniform_stack = wgpu_allocators::Stack::new(&uniform_heap);
    let camera = create_camera(
        &gfx,
        &mut command_encoder,
        &uniform_heap,
        &mut uniform_stack,
    );
    let mut cube = create_cube(
        &gfx,
        &mut command_encoder,
        &uniform_heap,
        &mut uniform_stack,
    );
    // We must unmap the uniform buffer before the command buffer can be submitted.
    uniform_heap.unmap();
    gfx.queue().submit(Some(command_encoder.finish()));

    let mut tick_count: f32 = 0.;
    let mut mouse_position = Point::ORIGIN;
    let mut mouse_is_down = false;
    let mut last_fps = 0;

    event_loop.run(move |event, _, ctrl_flow| {
        *ctrl_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        let [x, y] = [position.x, position.y].map(|coord| {
                            (((coord / (WINDOW_LENGTH as f64)) * 2.0) - 1.0) as f32
                        });
                        mouse_position.x = x;
                        mouse_position.y = y;
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        if matches!(button, MouseButton::Left) {
                            mouse_is_down = match state {
                                ElementState::Pressed => {
                                    tracing::info!("FPS: {}", last_fps);

                                    true
                                }
                                ElementState::Released => false,
                            };
                        }
                    }
                    WindowEvent::CloseRequested => {
                        *ctrl_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let tn = &mut cube.transforms_node;

                // Update cube position.
                {
                    let orbit_angle = tick_count / 100.0;
                    let position = tn.position_mut();
                    position.x = mouse_position.x + (orbit_angle.cos() / 10.0);
                    position.y = mouse_position.y + (orbit_angle.sin() / 10.0);
                }

                // Update cube rotation.
                {
                    let rotation_angle = tick_count / 10_000.0;
                    let rotation = tn.rotation_mut();
                    rotation.x = rotation_angle;
                    rotation.y = rotation_angle;
                }

                // Update cube scale.
                {
                    *tn.scale_mut() = if mouse_is_down {
                        0.5
                    } else {
                        0.05 + ((tick_count / 10_000.0).sin() + 1.0) / 50.0
                    };
                }

                tn.invalidate_cache();

                let mut command_encoder = gfx.device().create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: None },
                );
                // The cube's logical position, rotation, and scale has been modified&mdash;however,
                // the vertex shader doesn't know this yet. To convey this information to the GPU,
                // we must update the object transformation matrix, which is contained within
                // `uniform_heap`. And, of course, before we can write to `uniform_heap`, we need to
                // map it into CPU memory first.
                uniform_heap.map_range_async(
                    cube.transforms_range.clone(),
                    wgpu::MapMode::Write,
                );
                // As GPU buffer mapping is asynchronous, the buffer won't actually be mapped into
                // CPU memory until the device is polled. Here, the `wgpu::Maintain::Wait`
                // argument synchronously stalls the CPU until the buffer is mapped.
                gfx.device().poll(wgpu::Maintain::Wait);
                // With that setup out of the way, we can finally write the new transformation
                // matrix to the staging buffer and then immediately flush it to the GPU-local
                // buffer, which is what the vertex shader actually sees.
                uniform_heap.write_and_flush(
                    &mut command_encoder,
                    cube.transforms_range.clone(),
                    bytemuck::bytes_of(&tn.local_transformation_matrix().to_array()),
                );
                // I'm not really sure why the GPU can't do this for us, but *wgpu* will get upset
                // if our staging buffer is still mapped when the command buffer is submitted.
                uniform_heap.unmap();
                // And off our commands go!
                gfx.queue().submit(Some(command_encoder.finish()));

                let render = gfx.create_render();
                render.add_pass().x();
                render.submit();

                tick_count += 1.0;
                last_fps = fps_counter.tick()
            }
            _ => {}
        }
    });
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
}

fn create_window(event_loop: &EventLoop<()>) -> Window {
    WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_LENGTH, WINDOW_LENGTH))
        .with_resizable(false)
        .with_title("Cube")
        .build(event_loop)
        .expect("failed to build window")
}

fn create_gfx(window: &Window) -> Renderer {
    pollster::block_on(unsafe {
        Renderer::new(
            window,
            wgpu::Backends::all(),
            // In most cases, this should connect us to the discrete GPU if one is present, and the
            // integrated GPU otherwise.
            wgpu::PowerPreference::HighPerformance,
            pylon_engine::renderer::SurfaceSize {
                width: WINDOW_LENGTH as u32,
                height: WINDOW_LENGTH as u32,
            },
            // For meaningful FPS results, we'll disable V-sync.
            wgpu::PresentMode::AutoNoVsync,
        )
    })
    .unwrap()
}

fn create_camera(
    gfx: &Renderer,
    command_encoder: &mut wgpu::CommandEncoder,
    uniform_heap: &wgpu_allocators::Heap,
    uniform_stack: &mut wgpu_allocators::Stack,
) -> Camera {
    let transformation_matrix_range = uniform_stack.alloc(
        // SAFETY: The size of `[[f32; 4]; 4]` is nonzero.
        unsafe {
            NonZeroBufferAddress::new_unchecked(mem::size_of::<[[f32; 4]; 4]>() as u64)
        },
        // SAFETY: 256 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(256) },
    )
    .expect("transformation matrix allocation failed");

    let camera = Camera {
        transforms_uniform: gfx.create_camera_transforms_uniform(
            uniform_heap.binding(transformation_matrix_range.clone())
        ),
    };

    uniform_heap.write_and_flush(
        command_encoder,
        transformation_matrix_range,
        bytemuck::bytes_of(&camera.transformation_matrix().to_array()),
    );

    camera
}

struct Camera {
    transforms_uniform: CameraTransformsUniform,
}

impl pylon_engine::Camera for Camera {
    fn transforms_uniform(&self) -> &CameraTransformsUniform {
        &self.transforms_uniform
    }
}

impl Camera {
    fn transformation_matrix(&self) -> Matrix {
        Matrix::IDENTITY
    }
}

fn create_cube(
    gfx: &Renderer,
    command_encoder: &mut wgpu::CommandEncoder,
    uniform_heap: &wgpu_allocators::Heap,
    uniform_stack: &mut wgpu_allocators::Stack,
) -> Cube {
    let mesh = create_cube_mesh();

    let index_and_vertex_heap = wgpu_allocators::Heap::new(
        gfx.device(),
        // SAFETY: 512 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(512) },
        HeapUsages::INDEX | HeapUsages::VERTEX,
    );
    let mut index_and_vertex_stack = wgpu_allocators::Stack::new(&index_and_vertex_heap);

    let index_buffer_range = index_and_vertex_stack.alloc(
        // SAFETY: None of the terms are zero, so the product of them must be nonzero.
        unsafe {
            NonZeroBufferAddress::new_unchecked(
                (mem::size_of::<u32>() * 3 * mesh.triangles.len()) as u64,
            )
        },
        // SAFETY: 256 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(256) },
    )
    .expect("index buffer allocation failed");
    index_and_vertex_heap.write(
        index_buffer_range.clone(),
        bytemuck::cast_slice(&mesh.triangles),
    );

    let vertex_buffer_range = index_and_vertex_stack.alloc(
        // SAFETY: None of the terms are zero, so the product of them must be nonzero.
        unsafe {
            NonZeroBufferAddress::new_unchecked(
                (3 * mem::size_of::<f32>() * mesh.vertex_pool.len()) as u64,
            )
        },
        // SAFETY: 256 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(256) },
    )
    .expect("vertex buffer allocation failed");
    index_and_vertex_heap.write(
        vertex_buffer_range.clone(),
        bytemuck::cast_slice(&mesh.vertex_pool),
    );

    index_and_vertex_heap.flush(command_encoder);
    index_and_vertex_heap.unmap();

    let transforms_range = uniform_stack.alloc(
        // SAFETY: `ObjectTransforms` is not a ZST, so the size must be nonzero.
        unsafe {
            NonZeroBufferAddress::new_unchecked(mem::size_of::<[[f32; 4]; 4]>() as u64)
        },
        // SAFETY: 256 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(256) },
    )
    .expect("object transforms allocation failed");

    Cube {
        mesh,
        render_pipeline: gfx.create_pipeline(wgpu::ShaderSource::Wgsl(
            std::borrow::Cow::Borrowed(r#"
                @fragment
                fn main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
                    return vec4<f32>(
                        0.,
                        0.,
                        position.z * 2.0,
                        1.0,
                    );
                }
            "#)
        )),
        transforms_node: pylon_engine::tree::Node::default(),
        transforms_range: transforms_range.clone(),
        transforms_uniform: gfx.create_object_transforms_uniform(
            uniform_heap.binding(transforms_range)
        ),
        index_and_vertex_heap,
        index_buffer_range,
        vertex_buffer_range,
    }
}

fn create_cube_mesh() -> Mesh {
    Mesh {
        vertex_pool: vec![
            // 0.
            MeshVertex {
                // Left, lower, back.
                point: Point { x: -1., y: -1., z: -1. },
            },
            // 1.
            MeshVertex {
                // Left, lower, front.
                point: Point { x: -1., y: -1., z: 1. },
            },
            // 2.
            MeshVertex {
                // Left, upper, back.
                point: Point { x: -1., y: 1., z: -1. },
            },
            // 3.
            MeshVertex {
                // Left, upper, front.
                point: Point { x: -1., y: 1., z: 1. },
            },
            // 4.
            MeshVertex {
                // Right, lower, back.
                point: Point { x: 1., y: -1., z: -1. },
            },
            // 5.
            MeshVertex {
                // Right, lower, front.
                point: Point { x: 1., y: -1., z: 1. },
            },
            // 6.
            MeshVertex {
                // Right, upper, back.
                point: Point { x: 1., y: 1., z: -1. },
            },
            // 7.
            MeshVertex {
                // Right, upper, front.
                point: Point { x: 1., y: 1., z: 1. },
            },
        ],
        triangles: vec![
            // Left face.
            MeshTriangle::new([0, 1, 2]),
            MeshTriangle::new([1, 2, 3]),
            // Right face.
            MeshTriangle::new([4, 5, 6]),
            MeshTriangle::new([5, 6, 7]),
            // Lower face.
            MeshTriangle::new([0, 1, 4]),
            MeshTriangle::new([1, 4, 5]),
            // Upper face.
            MeshTriangle::new([2, 3, 6]),
            MeshTriangle::new([3, 6, 7]),
            // Back face.
            MeshTriangle::new([0, 2, 4]),
            MeshTriangle::new([2, 4, 6]),
            // Front face.
            MeshTriangle::new([1, 3, 5]),
            MeshTriangle::new([3, 5, 7]),
        ],
    }
}

struct Mesh {
    vertex_pool: Vec<MeshVertex>,
    triangles: Vec<MeshTriangle>,
}

struct Cube {
    /// The mesh.
    mesh: Mesh,
    /// The render pipeline for this cube.
    render_pipeline: wgpu::RenderPipeline,
    transforms_node: pylon_engine::tree::Node,
    /// The range of bytes in the uniform heap allocated to the transformation matrix for this cube.
    transforms_range: Range<BufferAddress>,
    /// The uniform for this cube's transformation matrix.
    transforms_uniform: ObjectTransformsUniform,
    /// The [heap](wgpu_allocators::Heap) containing the index and vertex buffers for this cube.
    index_and_vertex_heap: wgpu_allocators::Heap,
    /// The range of bytes in [`index_and_vertex_heap`](Self::index_and_vertex_heap) allocated to
    /// the index buffer for this cube.
    index_buffer_range: Range<BufferAddress>,
    /// The range of bytes in [`index_and_vertex_heap`](Self::index_and_vertex_heap) allocated to
    /// the vertex buffer for this cube.
    vertex_buffer_range: Range<BufferAddress>,
}

impl pylon_engine::Object for Cube {
    fn triangle_count(&self) -> u32 {
        self.mesh.triangles.len() as u32
    }

    fn render_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }

    fn transforms_uniform(&self) -> &ObjectTransformsUniform {
        &self.transforms_uniform
    }

    fn bind_group_slots<'a>(&'a self) -> &[BindGroupSlot<'a>] {
        // Our fragment shader is extremely simple and doesn't need any bind groups.
        &[]
    }

    fn index_buffer<'a>(&'a self) -> wgpu::BufferSlice<'a> {
        self.index_and_vertex_heap.slice(self.index_buffer_range.clone())
    }

    fn vertex_buffer<'a>(&'a self) -> wgpu::BufferSlice<'a> {
        self.index_and_vertex_heap.slice(self.vertex_buffer_range.clone())
    }
}
