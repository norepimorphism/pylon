use fps_counter::FPSCounter;
use pylon_engine::{
    Camera,
    Material,
    Mesh,
    MeshTriangle,
    MeshVertex,
    Object,
    ObjectTransforms,
    Point,
    Renderer,
    Rotation,
    Uniform,
};
use wgpu::BufferAddress;
use wgpu_allocators::{Allocator as _, HeapUsages, NonZeroBufferAddress};
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use std::{mem, ops::Range};

const WINDOW_LENGTH: f64 = 512.0;

fn main() {
    init_tracing();
    let mut fps_counter = FPSCounter::new();
    let event_loop = EventLoop::new();
    let window = create_window(&event_loop);

    let gfx = create_gfx(&window);
    let mut command_encoder = gfx.device().create_command_encoder(
        &wgpu::CommandEncoderDescriptor { label: None },
    );
    let uniform_heap = wgpu_allocators::Heap::new(
        gfx.device(),
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
                            (((coord / WINDOW_LENGTH) * 2.0) - 1.0) as f32
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
                // Update cube position.
                let orbit_angle = tick_count / 10.0;
                cube.position.x = mouse_position.x + (orbit_angle.cos() / 10.0);
                cube.position.y = mouse_position.y + (orbit_angle.sin() / 10.0);

                // Update cube rotation.
                cube.rotation.x += tick_count / 10_000.0;
                cube.rotation.y += tick_count / 10_000.0;

                // Update cube scale.
                cube.scale = if mouse_is_down {
                    0.1
                } else {
                    0.05 + ((tick_count / 10.0).sin() + 1.0) / 50.0
                };

                let mut command_encoder = gfx.device().create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: None },
                );
                uniform_heap.map_range_async(
                    cube.resources.transforms_range.clone(),
                    wgpu::MapMode::Write,
                );
                gfx.device().poll(wgpu::Maintain::Wait);
                uniform_heap.write_and_flush(
                    &mut command_encoder,
                    cube.resources.transforms_range.clone(),
                    bytemuck::bytes_of(&cube.transforms()),
                );
                uniform_heap.unmap();
                gfx.queue().submit(Some(command_encoder.finish()));

                gfx.render(&camera, [&cube]);

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
            wgpu::PowerPreference::HighPerformance,
            pylon_engine::renderer::SurfaceSize {
                width: WINDOW_LENGTH as u32,
                height: WINDOW_LENGTH as u32,
            },
        )
    })
    .unwrap()
}

fn create_camera(
    gfx: &Renderer,
    command_encoder: &mut wgpu::CommandEncoder,
    uniform_heap: &wgpu_allocators::Heap,
    uniform_stack: &mut wgpu_allocators::Stack,
) -> Camera<CameraResources> {
    let transformation_matrix_range = uniform_stack.alloc(
        // SAFETY: The size of `[[f32; 4]; 4]` is nonzero.
        unsafe {
            NonZeroBufferAddress::new_unchecked(mem::size_of::<[[f32; 4]; 4]>() as u64)
        },
        // SAFETY: 256 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(256) },
    )
    .expect("transformation matrix allocation failed");

    let resources = CameraResources {
        transformation_matrix: gfx.create_camera_transformation_matrix_uniform(
            uniform_heap.binding(transformation_matrix_range.clone())
        ),
    };

    let camera = Camera {
        position: Point::ORIGIN,
        target: Point::ORIGIN,
        roll: 1.,
        resources,
    };

    uniform_heap.write_and_flush(
        command_encoder,
        transformation_matrix_range,
        bytemuck::bytes_of(&camera.transformation_matrix().to_array()),
    );

    camera
}

struct CameraResources {
    transformation_matrix: Uniform,
}

impl pylon_engine::CameraResources for CameraResources {
    fn transformation_matrix_uniform(&self) -> &Uniform {
        &self.transformation_matrix
    }
}

fn create_cube(
    gfx: &Renderer,
    command_encoder: &mut wgpu::CommandEncoder,
    uniform_heap: &wgpu_allocators::Heap,
    uniform_stack: &mut wgpu_allocators::Stack,
) -> Object<CubeResources> {
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
            NonZeroBufferAddress::new_unchecked(mem::size_of::<ObjectTransforms>() as u64)
        },
        // SAFETY: 256 is nonzero.
        unsafe { NonZeroBufferAddress::new_unchecked(256) },
    )
    .expect("object transforms allocation failed");

    let resources = CubeResources {
        transforms_range: transforms_range.clone(),
        transforms: gfx.create_object_transforms_uniform(
            uniform_heap.binding(transforms_range)
        ),
        index_and_vertex_heap,
        index_buffer_range,
        vertex_buffer_range,
    };

    Object {
        position: Point::ORIGIN,
        rotation: Rotation::ZERO,
        scale: 1.,
        mesh,
        material: Material,
        resources,
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

struct CubeResources {
    transforms_range: Range<BufferAddress>,
    transforms: Uniform,
    index_and_vertex_heap: wgpu_allocators::Heap,
    index_buffer_range: Range<BufferAddress>,
    vertex_buffer_range: Range<BufferAddress>,
}

impl pylon_engine::ObjectResources for CubeResources {
    fn transforms_uniform(&self) -> &Uniform {
        &self.transforms
    }

    fn index_buffer<'a>(&'a self) -> wgpu::BufferSlice<'a> {
        self.index_and_vertex_heap.slice(self.index_buffer_range.clone())
    }

    fn vertex_buffer<'a>(&'a self) -> wgpu::BufferSlice<'a> {
        self.index_and_vertex_heap.slice(self.vertex_buffer_range.clone())
    }
}
