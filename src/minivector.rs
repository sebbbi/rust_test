#[derive(Clone, Debug, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug, Copy)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Debug, Copy)]
pub struct Mat4x4 {
    pub r0: Vec4,
    pub r1: Vec4,
    pub r2: Vec4,
    pub r3: Vec4,
}

fn mul(a: Mat4x4, b: Mat4x4) -> Mat4x4 {
    Mat4x4 {
        r0: Vec4 {
            x: a.r0.x * b.r0.x + a.r0.y * b.r1.x + a.r0.z * b.r2.x + a.r0.w * b.r3.x,
            y: a.r0.x * b.r0.y + a.r0.y * b.r1.y + a.r0.z * b.r2.y + a.r0.w * b.r3.y,
            z: a.r0.x * b.r0.z + a.r0.y * b.r1.z + a.r0.z * b.r2.z + a.r0.w * b.r3.z,
            w: a.r0.x * b.r0.w + a.r0.y * b.r1.w + a.r0.z * b.r2.w + a.r0.w * b.r3.w,
        },
        r1: Vec4 {
            x: a.r1.x * b.r0.x + a.r1.y * b.r1.x + a.r1.z * b.r2.x + a.r1.w * b.r3.x,
            y: a.r1.x * b.r0.y + a.r1.y * b.r1.y + a.r1.z * b.r2.y + a.r1.w * b.r3.y,
            z: a.r1.x * b.r0.z + a.r1.y * b.r1.z + a.r1.z * b.r2.z + a.r1.w * b.r3.z,
            w: a.r1.x * b.r0.w + a.r1.y * b.r1.w + a.r1.z * b.r2.w + a.r1.w * b.r3.w,
        },
        r2: Vec4 {
            x: a.r2.x * b.r0.x + a.r2.y * b.r1.x + a.r2.z * b.r2.x + a.r2.w * b.r3.x,
            y: a.r2.x * b.r0.y + a.r2.y * b.r1.y + a.r2.z * b.r2.y + a.r2.w * b.r3.y,
            z: a.r2.x * b.r0.z + a.r2.y * b.r1.z + a.r2.z * b.r2.z + a.r2.w * b.r3.z,
            w: a.r2.x * b.r0.w + a.r2.y * b.r1.w + a.r2.z * b.r2.w + a.r2.w * b.r3.w,
        },
        r3: Vec4 {
            x: a.r3.x * b.r0.x + a.r3.y * b.r1.x + a.r3.z * b.r2.x + a.r3.w * b.r3.x,
            y: a.r3.x * b.r0.y + a.r3.y * b.r1.y + a.r3.z * b.r2.y + a.r3.w * b.r3.y,
            z: a.r3.x * b.r0.z + a.r3.y * b.r1.z + a.r3.z * b.r2.z + a.r3.w * b.r3.z,
            w: a.r3.x * b.r0.w + a.r3.y * b.r1.w + a.r3.z * b.r2.w + a.r3.w * b.r3.w,
        },
    }
}
