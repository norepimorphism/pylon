use fps_counter::FPSCounter;
use pylon_engine::*;
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const WINDOW_SIZE: f64 = 512.0;

fn main() {
    init_tracing();
    let mut fps_counter = FPSCounter::new();
    let event_loop = EventLoop::new();
    let window = create_window(&event_loop);
    let mut gfx = create_gfx(&window);
    let camera = create_camera();
    let mut cube = create_cube();

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
                            (((coord / WINDOW_SIZE) * 2.0) - 1.0) as f32
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
                let position = cube.position_mut();
                position.x = mouse_position.x + (orbit_angle.cos() / 10.0);
                position.y = mouse_position.y + (orbit_angle.sin() / 10.0);

                // Update cube rotation.
                let rotation = cube.rotation_mut();
                rotation.x += tick_count / 10_000.0;
                rotation.y += tick_count / 10_000.0;

                // Update cube scale.
                *cube.scale_mut() = if mouse_is_down {
                    0.1
                } else {
                    0.05 + ((tick_count / 10.0).sin() + 1.0) / 50.0
                };

                gfx.render(&camera, [&mut cube].into_iter());

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
        .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_SIZE, WINDOW_SIZE))
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
            WINDOW_SIZE as u32,
            WINDOW_SIZE as u32,
        )
    })
    .unwrap()
}

fn create_camera() -> Camera {
    Camera {
        position: Point::ORIGIN,
        target: Point::ORIGIN,
        roll: 1.,
    }
}

fn create_cube() -> Object {
    Object::new(
        Point::ORIGIN,
        Rotation::ZERO,
        0.05,
        Material,
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
        },
    )
}
