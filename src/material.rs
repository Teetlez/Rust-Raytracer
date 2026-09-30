use ultraviolet::Vec3;

#[derive(Debug, Copy, Clone)]
pub struct Lambertian {
    pub albedo: Vec3,
}

#[derive(Debug, Copy, Clone)]
pub struct Glossy {
    pub albedo: Vec3,
    pub reflectance: f32,
    pub roughness: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct Metal {
    pub albedo: Vec3,
    pub roughness: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct Dielectric {
    pub albedo: Vec3,
    pub refractive_index: f32,
    pub roughness: f32,
}

#[derive(Debug, Copy, Clone)]
pub enum Material {
    Dielectric(Dielectric),
    Lambertian(Lambertian),
    Metal(Metal),
    Glossy(Glossy),
}

impl Material {
    pub fn lambertian(albedo: (f32, f32, f32)) -> Self {
        Self::Lambertian(Lambertian {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
        })
    }

    pub fn glossy(albedo: (f32, f32, f32), reflectance: f32, roughness: f32) -> Self {
        Self::Glossy(Glossy {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            reflectance,
            roughness,
        })
    }

    pub fn metal(albedo: (f32, f32, f32), roughness: f32) -> Self {
        Self::Metal(Metal {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            roughness,
        })
    }

    pub fn dielectric(albedo: (f32, f32, f32), refractive_index: f32, roughness: f32) -> Self {
        Self::Dielectric(Dielectric {
            albedo: Vec3::new(albedo.0, albedo.1, albedo.2),
            refractive_index,
            roughness,
        })
    }
}