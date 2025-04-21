use anyhow::Ok;
use glam::{vec3, vec4, Vec3, Vec4};
use std::{f32::consts::FRAC_PI_2, fs, io::Write};

#[derive(Clone, Copy, Debug, Default)]
struct Material {
    pub color: Vec3,
    pub albedo: Vec4,
    pub specular: f32,
    pub refract_index: f32,
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

#[derive(Debug, Clone, Copy)]
struct PointLight {
    position: Vec3,
    intensity: f32,
}

impl PointLight {
    pub fn new(position: Vec3, intensity: f32) -> Self {
        Self {
            position,
            intensity,
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

fn refract(i: &Vec3, n: &Vec3, refract_index: &f32) -> Vec3 {
    let mut cosi = -i.dot(*n).clamp(-1., 1.);
    let mut etai = 1.;
    let mut etat = *refract_index;
    let mut n = n.clone();
    if cosi < 0. {
        cosi = -cosi;
        std::mem::swap(&mut etai, &mut etat);
        n = -n;
    }
    let eta = etai / etat;
    let k = 1. - eta * eta * (1. - cosi * cosi);

    if k < 0. {
        Vec3::ZERO
    } else {
        (i * eta + n * (eta * cosi - k.sqrt())).normalize()
    }
}

fn scene_intersect(ray: &Ray, spheres: &[Sphere]) -> Option<(Vec3, Vec3, Material)> {
    let mut dist = f32::MAX;
    let mut hit = Vec3::ZERO;
    let mut normal = Vec3::X;
    let mut material = Material::default();

    for s in spheres {
        let intersect_test_res = s.ray_intersect(&ray);
        if intersect_test_res.0 && intersect_test_res.1 < dist {
            dist = intersect_test_res.1;
            hit = ray.origin + ray.direction * dist;
            normal = (hit - s.center).normalize();
            material = s.material;
        }
    }

    let mut checkerboard_dist = f32::MAX;
    if ray.direction.y.abs() > 1e-3 {
        let d = -(ray.origin.y + 4.) / ray.direction.y; // the checkerboard plane has equation y = -4
        let pt = ray.origin + ray.direction * d;
        if d > 0. && pt.x.abs() < 10. && pt.z < -10. && pt.z > -30. && d < dist {
            checkerboard_dist = d;
            hit = pt;
            normal = Vec3::Y;
            material.color =
                if ((0.5 * hit.x + 1000.0) as i32 + (0.5 * hit.z).round() as i32) % 2 == 1 {
                    vec3(1., 1., 1.)
                } else {
                    vec3(1., 0.7, 0.3)
                };
            material.color = material.color * 0.3;
            material.albedo = Vec4::X;
            material.refract_index = 1.0;
            material.specular = 0.0;
        }
    }

    if dist.min(checkerboard_dist) < 1000. {
        Some((hit, normal, material))
    } else {
        None
    }
}

fn cast_ray(ray: &Ray, spheres: &[Sphere], lights: &[PointLight], depth: usize) -> Vec3 {
    const BACKGROUND: Vec3 = vec3(0.2, 0.7, 0.8);

    if depth > 4 {
        return BACKGROUND;
    }

    if let Some((point, normal, material)) = scene_intersect(ray, spheres) {
        let reflect_dir = ray.direction.reflect(normal).normalize();
        let refract_dir = refract(&ray.direction, &normal, &material.refract_index);
        let reflect_origin = if reflect_dir.dot(normal) < 0. {
            point - normal * 1e-3
        } else {
            point + normal * 1e-3
        };
        let refract_origin = if refract_dir.dot(normal) < 0. {
            point - normal * 1e-3
        } else {
            point + normal * 1e-3
        };
        let reflect_color = cast_ray(
            &Ray {
                origin: reflect_origin,
                direction: reflect_dir,
            },
            spheres,
            lights,
            depth + 1,
        );
        let refract_color = if reflect_dir.length() == 0. {
            Vec3::ZERO
        } else {
            cast_ray(
                &Ray {
                    origin: refract_origin,
                    direction: refract_dir,
                },
                spheres,
                lights,
                depth + 1,
            )
        };

        let mut diffuse_intensity = 0.;
        let mut specular_intensity = 0.;

        for light in lights {
            let light_dir = (light.position - point).normalize();
            let light_distence = (light.position - point).length();

            let shadow_origin = if light_dir.dot(normal) < 0. {
                point - normal * 1e-3
            } else {
                point + normal * 1e-3
            };
            let mut shadowed = false;
            if let Some((hit, _, _)) = scene_intersect(
                &Ray {
                    origin: shadow_origin,
                    direction: light_dir,
                },
                spheres,
            ) {
                if (hit - shadow_origin).length() < light_distence {
                    shadowed = true;
                }
            }
            if shadowed {
                continue;
            }

            diffuse_intensity += light.intensity * light_dir.dot(normal).max(0.);
            specular_intensity += light.intensity
                * (-light_dir)
                    .reflect(normal)
                    .dot(-ray.direction)
                    .max(0.)
                    .powf(material.specular);
        }

        let color = material.color * diffuse_intensity * material.albedo.x
            + specular_intensity * material.albedo.y
            + reflect_color * material.albedo.z
            + refract_color * material.albedo.w;
        color / color.max_element().max(1.)
    } else {
        BACKGROUND
    }
}

fn render(spheres: &[Sphere], lights: &[PointLight]) -> anyhow::Result<(Vec<Vec3>, usize, usize)> {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = 768;

    const FOV: f32 = 1.05;

    let mut framebuffer = vec![Vec3::ZERO; WIDTH * HEIGHT];

    for j in 0..HEIGHT {
        for i in 0..WIDTH {
            let x = (2.0 * (i as f32 + 0.5) / WIDTH as f32 - 1.0) * (FOV / 2.).tan() * WIDTH as f32
                / HEIGHT as f32;
            let y = -(2.0 * (j as f32 + 0.5) / HEIGHT as f32 - 1.0) * (FOV / 2.).tan();
            let ray = Ray::new(vec3(-0.0, -0.0, 0.), vec3(x, y, -1.0));

            framebuffer[i + j * WIDTH] = cast_ray(&ray, spheres, lights, 0);
        }
    }

    Ok((framebuffer, WIDTH, HEIGHT))
}

fn main() -> anyhow::Result<()> {
    let ivory = Material {
        color: vec3(0.4, 0.4, 0.3),
        albedo: vec4(0.6, 0.3, 0.1, 0.0),
        specular: 50.,
        refract_index: 1.0,
    };
    let glass = Material {
        color: vec3(0.6, 0.7, 0.8),
        albedo: vec4(0.0, 0.5, 0.1, 0.8),
        specular: 125.,
        refract_index: 1.5,
    };
    let red_rubber = Material {
        color: vec3(0.3, 0.1, 0.1),
        albedo: vec4(0.9, 0.1, 0.0, 0.0),
        specular: 10.,
        refract_index: 1.0,
    };
    let mirror = Material {
        color: vec3(1.0, 1.0, 1.0),
        albedo: vec4(0.0, 10.0, 0.8, 0.0),
        specular: 1425.,
        refract_index: 1.0,
    };
    let spheres = vec![
        Sphere::new(vec3(-3., 0., -16.), 2., ivory),
        Sphere::new(vec3(-1.0, -1.5, -12.), 2., glass),
        Sphere::new(vec3(1.5, -0.5, -18.), 3., red_rubber),
        Sphere::new(vec3(7., 5., -18.), 4., mirror),
    ];
    let lights = vec![
        PointLight::new(vec3(-20., 20., 20.), 1.5),
        PointLight::new(vec3(30., 50., -25.), 1.8),
        PointLight::new(vec3(30., 20., 30.), 1.7),
    ];

    let (framebuffer, width, height) = render(&spheres, &lights)?;

    let mut f = fs::File::create("./out.ppm")?;
    write!(f, "P6\n{} {}\n255\n", width, height)?;
    f.write(&map_to_image(&framebuffer))?;

    Ok(())
}
