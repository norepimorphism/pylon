use pylon_engine::*;

use std::{thread, time::Duration};

const WINDOW_SIZE: usize = 512;

fn main() {
    init_tracing();
    let mut window = create_window();
    let mut gfx = create_gfx(&window);
    let camera = create_camera();
    let mut cube = create_cube();

    let mut tick_count: f32 = 0.;
    let (mut x, mut y) = (0., 0.);

    loop {
        // Get mouse position.
        if let Some([mx, my]) = window
            .get_mouse_pos(minifb::MouseMode::Discard)
            .map(|(x, y)| {
                [x, y].map(|coord| ((coord / 512.0) * 2.0) - 1.0)
            })
        {
            x = mx;
            y = my;
        }

        // Update cube position.
        let orbit_angle = tick_count / 10.0;
        let position = cube.position_mut();
        position.x = x + (orbit_angle.cos() / 10.0);
        position.y = y + (orbit_angle.sin() / 10.0);

        cube.rotation_mut().x += tick_count / 10_000.0;
        cube.rotation_mut().y += tick_count / 10_000.0;
        if window.get_mouse_down(minifb::MouseButton::Left) {
            *cube.scale_mut() = 0.1;
        } else {
            *cube.scale_mut() = 0.05 + ((tick_count / 10.0).sin() + 1.0) / 50.0;
        }

        tick_count += 1.0;
        window.update();
        gfx.render(&camera, [&mut cube].into_iter());

        thread::sleep(Duration::from_millis(5));
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
}

fn create_window() -> minifb::Window {
    minifb::Window::new(
        "Cube",
        WINDOW_SIZE,
        WINDOW_SIZE,
        minifb::WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            ..Default::default()
        },
    )
    .unwrap()
}

fn create_gfx(window: &minifb::Window) -> Renderer {
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
