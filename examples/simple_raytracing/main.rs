use anyhow::Ok;
use glam::{vec3, Vec3};
use std::{fs, io::Write};

#[derive(Clone, Copy, Debug)]
struct Material {
    pub color: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct Sphere {
    center: Vec3,
    radius: f32,
    material: Material,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32, material: Material) -> Self {
        Self {
            center,
            radius,
            material,
        }
    }

    pub fn ray_intersect(&self, ray: &Ray) -> (bool, f32) {
        let o2c = self.center - ray.origin;
        let lcos = o2c.dot(ray.direction);
        let d2 = o2c.length_squared() - lcos * lcos;

        let x = self.radius * self.radius - d2;
        if x < 0. {
            (false, f32::MAX)
        } else {
            let y = x.sqrt();
            let t0 = lcos - y;
            let t1 = lcos + y;
            if t0 < 0. {
                (false, t1)
            } else {
                (true, t0)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }
}

fn map_to_image(frame_buffer: &[Vec3]) -> Vec<u8> {
    frame_buffer
        .iter()
        .map(|v| {
            v.to_array()
                .map(|x| (x.clamp(0., 1.) * 255.) as u8)
                .to_vec()
        })
        .flatten()
        .collect::<Vec<u8>>()
}

fn render(sphere: &[Sphere]) -> anyhow::Result<(Vec<Vec3>, usize, usize)> {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = 768;

    const FOV: f32 = 1.05;

    const BACKGROUND: Vec3 = vec3(0.2, 0.7, 0.8);

    let mut framebuffer = vec![Vec3::ZERO; WIDTH * HEIGHT];

    for j in 0..HEIGHT {
        for i in 0..WIDTH {
            let x = (2.0 * (i as f32 + 0.5) / WIDTH as f32 - 1.0) * (FOV / 2.).tan() * WIDTH as f32
                / HEIGHT as f32;
            let y = -(2.0 * (j as f32 + 0.5) / HEIGHT as f32 - 1.0) * (FOV / 2.).tan();
            let ray = Ray::new(Vec3::ZERO, vec3(x, y, -1.0));

            let mut intersect_length = f32::MAX;
            let mut color = BACKGROUND;
            for s in sphere {
                let intersect_test_res = s.ray_intersect(&ray);
                if intersect_test_res.0 && intersect_test_res.1 < intersect_length {
                    intersect_length = intersect_test_res.1;
                    color = s.material.color;
                }
            }

            framebuffer[i + j * WIDTH] = color;
        }
    }

    Ok((framebuffer, WIDTH, HEIGHT))
}

fn main() -> anyhow::Result<()> {
    let ivory = Material {
        color: vec3(0.4, 0.4, 0.3),
    };
    let red_rubber = Material {
        color: vec3(0.3, 0.1, 0.1),
    };
    let spheres = vec![
        Sphere::new(vec3(-3., 0., -16.), 2., ivory),
        Sphere::new(vec3(-1.0, -1.5, -12.), 2., red_rubber),
        Sphere::new(vec3(1.5, -0.5, -18.), 3., red_rubber),
        Sphere::new(vec3(7., 5., -18.), 4., ivory),
    ];

    let (framebuffer, width, height) = render(&spheres)?;

    let mut f = fs::File::create("./out.ppm")?;
    write!(f, "P6\n{} {}\n255\n", width, height)?;
    f.write(&map_to_image(&framebuffer))?;

    Ok(())
}
