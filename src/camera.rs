use ultraviolet::Vec3;

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    eye: Vec3,
    lookat: Vec3,
    fov: f32,
    aspect_ratio: f32,
    aperture: f32,
    focus_dist: f32,
}

impl Camera {
    pub fn new(
        eye: Vec3,
        lookat: Vec3,
        fov: f32,
        aspect_ratio: f32,
        apeture: f32,
        focus_dist: f32,
    ) -> Camera {
        Camera {
            eye,
            lookat,
            fov,
            aspect_ratio,
            aperture: apeture,
            focus_dist,
        }
    }

    pub fn gpu_parameters(&self) -> (Vec3, Vec3, f32, f32, f32, f32) {
        (
            self.eye,
            (self.lookat - self.eye).normalized(),
            self.fov,
            self.aspect_ratio,
            self.aperture,
            self.focus_dist,
        )
    }
}
