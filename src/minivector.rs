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

fn projection(fovy: f32, aspect: f32, znear: f32, zfar: f32) -> Mat4x4 {
    let h = 1.0 / (fovy * 0.5).tan();
    let w = h / aspect;
    let a = zfar / (zfar - znear);
    let b = (-znear * zfar) / (zfar - znear);

    Mat4x4 {
        r0: Vec4 {
            x: w,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: h,
            z: 0.0,
            w: 0.0,
        },
        r2: Vec4 {
            x: 0.0,
            y: 0.0,
            z: a,
            w: b,
        },
        r3: Vec4 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            w: 0.0,
        },
    }
}

fn rot_x_axis(r: f32) -> Mat4x4 {
    Mat4x4 {
        r0: Vec4 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: r.cos(),
            z: r.sin(),
            w: 0.0,
        },
        r2: Vec4 {
            x: 0.0,
            y: -r.sin(),
            z: r.cos(),
            w: 0.0,
        },
        r3: Vec4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
    }
}

fn rot_y_axis(r: f32) -> Mat4x4 {
    Mat4x4 {
        r0: Vec4 {
            x: r.cos(),
            y: 0.0,
            z: -r.sin(),
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            w: 0.0,
        },
        r2: Vec4 {
            x: r.sin(),
            y: 0.0,
            z: r.cos(),
            w: 0.0,
        },
        r3: Vec4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
    }
}

fn rot_z_axis(r: f32) -> Mat4x4 {
    Mat4x4 {
        r0: Vec4 {
            x: r.cos(),
            y: r.sin(),
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: -r.sin(),
            y: r.cos(),
            z: 0.0,
            w: 0.0,
        },
        r2: Vec4 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            w: 0.0,
        },
        r3: Vec4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
    }
}
