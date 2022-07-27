use pylon_engine::*;

use std::{thread, time::Duration};

fn main() {
    const WIDTH: usize = 512;
    const HEIGHT: usize = 512;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let mut window = minifb::Window::new(
        "Cube",
        WIDTH,
        HEIGHT,
        minifb::WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            ..Default::default()
        },
    )
    .unwrap();

    let mut gfx = pollster::block_on(unsafe {
        Renderer::new(
            &window,
            wgpu::Backends::all(),
            WIDTH as u32,
            HEIGHT as u32,
        )
    })
    .unwrap();

    let mut scene = scene();
    let mut tick_count: f32 = 0.;
    loop {
        window.update();
        gfx.render(&scene);

        let cube = scene.objects.first_mut().unwrap();
        cube.scale = 1.0 + ((tick_count / 10.0).sin() + 1.0) / 4.0;

        tick_count += 1.0;
        thread::sleep(Duration::from_millis(10));
    }
}

fn scene() -> Scene {
    Scene {
        camera: Camera {
            position: Point::ORIGIN,
            target: Point::ORIGIN,
            roll: 1.,
        },
        objects: vec![Object {
            position: Point::ORIGIN,
            scale: 1.,
            mesh: Mesh {
                vertex_pool: vec![
                    // 0.
                    MeshVertex {
                        // Left, lower, back.
                        point: Point { x: -0.5, y: -0.5, z: -0.5 },
                    },
                    // 1.
                    MeshVertex {
                        // Left, lower, front.
                        point: Point { x: -0.5, y: -0.5, z: 0.5 },
                    },
                    // 2.
                    MeshVertex {
                        // Left, upper, back.
                        point: Point { x: -0.5, y: 0.5, z: -0.5 },
                    },
                    // 3.
                    MeshVertex {
                        // Left, upper, front.
                        point: Point { x: -0.5, y: 0.5, z: 0.5 },
                    },
                    // 4.
                    MeshVertex {
                        // Right, lower, back.
                        point: Point { x: 0.5, y: -0.5, z: -0.5 },
                    },
                    // 5.
                    MeshVertex {
                        // Right, lower, front.
                        point: Point { x: 0.5, y: -0.5, z: 0.5 },
                    },
                    // 6.
                    MeshVertex {
                        // Right, upper, back.
                        point: Point { x: 0.5, y: 0.5, z: -0.5 },
                    },
                    // 7.
                    MeshVertex {
                        // Right, upper, front.
                        point: Point { x: 0.5, y: 0.5, z: 0.5 },
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
            material: Material,
        }],
    }
}
