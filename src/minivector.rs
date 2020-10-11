use std::ops;

#[derive(Clone, Debug, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn to_4d(self) -> Vec4 {
        Vec4 {
            x: self.x,
            y: self.y,
            z: self.z,
            w: 1.0,
        }
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z 
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: (self.y * other.z) - (self.z * other.y),
            y: (self.z * other.x) - (self.x * other.z),
            z: (self.x * other.y) - (self.y * other.x),
        }
    }

    pub fn normalize(self) -> Vec3 {
        let l2 = (self.x * self.x + self.y * self.y + self.z * self.z);
        let l_inv = 1.0 / l2.sqrt();
        Vec3 { x: self.x * l_inv, y: self.y * l_inv, z: self.z * l_inv}
	}
}

impl ops::Add<Vec3> for Vec3 {
    type Output = Vec3;

    fn add(self, _rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + _rhs.x,
            y: self.y + _rhs.y,
            z: self.z + _rhs.z,
        }
    }
}

impl ops::Sub<Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(self, _rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - _rhs.x,
            y: self.y - _rhs.y,
            z: self.z - _rhs.z,
        }
    }
}

impl ops::Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn to_3d(self) -> Vec3 {
        Vec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }

    pub fn normalize(self) -> Vec4 {
        let l2 = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w);
        let l_inv = 1.0 / l2.sqrt();
        Vec4 { x: self.x * l_inv, y: self.y * l_inv, z: self.z * l_inv, w: self.w * l_inv }
	}
}

#[derive(Clone, Debug, Copy)]
pub struct Mat4x4 {
    pub r0: Vec4,
    pub r1: Vec4,
    pub r2: Vec4,
    pub r3: Vec4,
}

pub fn mul(a: Mat4x4, b: Mat4x4) -> Mat4x4 {
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

pub fn identity() -> Mat4x4 {
    Mat4x4 {
        r0: Vec4 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: 1.0,
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

pub fn view(position: Vec3, forward: Vec3, up: Vec3) -> Mat4x4 {
    let forward = forward.normalize();
    let right = up.cross(forward).normalize();
    let up = forward.cross(right).normalize();

    Mat4x4 {
        r0: Vec4 {
            x: right.x,
            y: up.x,
            z: forward.x,
            w: 0.0,
        },
        r1: Vec4 {
            x: right.y,
            y: up.y,
            z: forward.y,
            w: 0.0,
        },
        r2: Vec4 {
            x: right.z,
            y: up.z,
            z: forward.z,
            w: 0.0,
        },
        r3: Vec4 {
            x: -position.dot(right),
            y: -position.dot(up),
            z: -position.dot(forward),
            w: 1.0,
        },
    }
}

pub fn projection(fovy: f32, aspect: f32, znear: f32, zfar: f32) -> Mat4x4 {
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
            w: 1.0,
        },
        r3: Vec4 {
            x: 0.0,
            y: 0.0,
            z: b,
            w: 0.0,
        },
    }
}

pub fn projection2(fovy: f32, aspect: f32, znear: f32, zfar: f32) -> Mat4x4 {
    let f = 1.0 / (fovy * 0.5).tan();

    Mat4x4 {
        r0: Vec4 {
            x: f / aspect,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: -f,
            z: 0.0,
            w: 0.0,
        },
        r2: Vec4 {
            x: 0.0,
            y: 0.0,
            z: zfar / (znear - zfar),
            w: -1.0,
        },
        r3: Vec4 {
            x: 0.0,
            y: 0.0,
            z: (znear * zfar) / (znear - zfar),
            w: 0.0,
        },
    }
}

pub fn translate(position: Vec3) -> Mat4x4 {
    Mat4x4 {
        r0: Vec4 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            w: 0.0,
        },
        r2: Vec4 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            w: 0.0,
        },
        r3: position.to_4d(),
    }
}

pub fn rot_x_axis(r: f32) -> Mat4x4 {
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

pub fn rot_y_axis(r: f32) -> Mat4x4 {
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

pub fn rot_z_axis(r: f32) -> Mat4x4 {
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
