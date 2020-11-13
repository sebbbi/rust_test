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

    pub fn from_scalar(v: f32) -> Vec3 {
        Vec3 { x: v, y: v, z: v }
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

    pub fn length(self) -> f32 {
        let l2 = self.x * self.x + self.y * self.y + self.z * self.z;
        l2.sqrt()
    }

    pub fn normalize(self) -> Vec3 {
        let l2 = self.x * self.x + self.y * self.y + self.z * self.z;
        let l_inv = 1.0 / l2.sqrt();
        Vec3 {
            x: self.x * l_inv,
            y: self.y * l_inv,
            z: self.z * l_inv,
        }
    }
}

impl ops::Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, _rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x * _rhs,
            y: self.y * _rhs,
            z: self.z * _rhs,
        }
    }
}

impl ops::Mul<Vec3> for Vec3 {
    type Output = Vec3;

    fn mul(self, _rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x * _rhs.x,
            y: self.y * _rhs.y,
            z: self.z * _rhs.z,
        }
    }
}

impl ops::Div<Vec3> for Vec3 {
    type Output = Vec3;

    fn div(self, _rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x / _rhs.x,
            y: self.y / _rhs.y,
            z: self.z / _rhs.z,
        }
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
        let l2 = self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
        let l_inv = 1.0 / l2.sqrt();
        Vec4 {
            x: self.x * l_inv,
            y: self.y * l_inv,
            z: self.z * l_inv,
            w: self.w * l_inv,
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Mat4x4 {
    pub r0: Vec4,
    pub r1: Vec4,
    pub r2: Vec4,
    pub r3: Vec4,
}

impl ops::Mul<Mat4x4> for Mat4x4 {
    type Output = Mat4x4;

    fn mul(self, _rhs: Mat4x4) -> Mat4x4 {
        Mat4x4 {
            r0: Vec4 {
                x: self.r0.x * _rhs.r0.x
                    + self.r0.y * _rhs.r1.x
                    + self.r0.z * _rhs.r2.x
                    + self.r0.w * _rhs.r3.x,
                y: self.r0.x * _rhs.r0.y
                    + self.r0.y * _rhs.r1.y
                    + self.r0.z * _rhs.r2.y
                    + self.r0.w * _rhs.r3.y,
                z: self.r0.x * _rhs.r0.z
                    + self.r0.y * _rhs.r1.z
                    + self.r0.z * _rhs.r2.z
                    + self.r0.w * _rhs.r3.z,
                w: self.r0.x * _rhs.r0.w
                    + self.r0.y * _rhs.r1.w
                    + self.r0.z * _rhs.r2.w
                    + self.r0.w * _rhs.r3.w,
            },
            r1: Vec4 {
                x: self.r1.x * _rhs.r0.x
                    + self.r1.y * _rhs.r1.x
                    + self.r1.z * _rhs.r2.x
                    + self.r1.w * _rhs.r3.x,
                y: self.r1.x * _rhs.r0.y
                    + self.r1.y * _rhs.r1.y
                    + self.r1.z * _rhs.r2.y
                    + self.r1.w * _rhs.r3.y,
                z: self.r1.x * _rhs.r0.z
                    + self.r1.y * _rhs.r1.z
                    + self.r1.z * _rhs.r2.z
                    + self.r1.w * _rhs.r3.z,
                w: self.r1.x * _rhs.r0.w
                    + self.r1.y * _rhs.r1.w
                    + self.r1.z * _rhs.r2.w
                    + self.r1.w * _rhs.r3.w,
            },
            r2: Vec4 {
                x: self.r2.x * _rhs.r0.x
                    + self.r2.y * _rhs.r1.x
                    + self.r2.z * _rhs.r2.x
                    + self.r2.w * _rhs.r3.x,
                y: self.r2.x * _rhs.r0.y
                    + self.r2.y * _rhs.r1.y
                    + self.r2.z * _rhs.r2.y
                    + self.r2.w * _rhs.r3.y,
                z: self.r2.x * _rhs.r0.z
                    + self.r2.y * _rhs.r1.z
                    + self.r2.z * _rhs.r2.z
                    + self.r2.w * _rhs.r3.z,
                w: self.r2.x * _rhs.r0.w
                    + self.r2.y * _rhs.r1.w
                    + self.r2.z * _rhs.r2.w
                    + self.r2.w * _rhs.r3.w,
            },
            r3: Vec4 {
                x: self.r3.x * _rhs.r0.x
                    + self.r3.y * _rhs.r1.x
                    + self.r3.z * _rhs.r2.x
                    + self.r3.w * _rhs.r3.x,
                y: self.r3.x * _rhs.r0.y
                    + self.r3.y * _rhs.r1.y
                    + self.r3.z * _rhs.r2.y
                    + self.r3.w * _rhs.r3.y,
                z: self.r3.x * _rhs.r0.z
                    + self.r3.y * _rhs.r1.z
                    + self.r3.z * _rhs.r2.z
                    + self.r3.w * _rhs.r3.z,
                w: self.r3.x * _rhs.r0.w
                    + self.r3.y * _rhs.r1.w
                    + self.r3.z * _rhs.r2.w
                    + self.r3.w * _rhs.r3.w,
            },
        }
    }
}

impl ops::Mul<Mat4x4> for Vec3 {
    type Output = Vec3;

    fn mul(self, _rhs: Mat4x4) -> Vec3 {
        Vec3 {
            x: self.x * _rhs.r0.x + self.y * _rhs.r1.x + self.z * _rhs.r2.x + _rhs.r3.x,
            y: self.x * _rhs.r0.y + self.y * _rhs.r1.y + self.z * _rhs.r2.y + _rhs.r3.y,
            z: self.x * _rhs.r0.z + self.y * _rhs.r1.z + self.z * _rhs.r2.z + _rhs.r3.z,
        }
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

pub fn inverse(m: Mat4x4) -> Mat4x4 {
    let a2323 = m.r2.z * m.r3.w - m.r2.w * m.r3.z;
    let a1323 = m.r2.y * m.r3.w - m.r2.w * m.r3.y;
    let a1223 = m.r2.y * m.r3.z - m.r2.z * m.r3.y;
    let a0323 = m.r2.x * m.r3.w - m.r2.w * m.r3.x;
    let a0223 = m.r2.x * m.r3.z - m.r2.z * m.r3.x;
    let a0123 = m.r2.x * m.r3.y - m.r2.y * m.r3.x;
    let a2313 = m.r1.z * m.r3.w - m.r1.w * m.r3.z;
    let a1313 = m.r1.y * m.r3.w - m.r1.w * m.r3.y;
    let a1213 = m.r1.y * m.r3.z - m.r1.z * m.r3.y;
    let a2312 = m.r1.z * m.r2.w - m.r1.w * m.r2.z;
    let a1312 = m.r1.y * m.r2.w - m.r1.w * m.r2.y;
    let a1212 = m.r1.y * m.r2.z - m.r1.z * m.r2.y;
    let a0313 = m.r1.x * m.r3.w - m.r1.w * m.r3.x;
    let a0213 = m.r1.x * m.r3.z - m.r1.z * m.r3.x;
    let a0312 = m.r1.x * m.r2.w - m.r1.w * m.r2.x;
    let a0212 = m.r1.x * m.r2.z - m.r1.z * m.r2.x;
    let a0113 = m.r1.x * m.r3.y - m.r1.y * m.r3.x;
    let a0112 = m.r1.x * m.r2.y - m.r1.y * m.r2.x;

    let det = m.r0.x * (m.r1.y * a2323 - m.r1.z * a1323 + m.r1.w * a1223)
        - m.r0.y * (m.r1.x * a2323 - m.r1.z * a0323 + m.r1.w * a0223)
        + m.r0.z * (m.r1.x * a1323 - m.r1.y * a0323 + m.r1.w * a0123)
        - m.r0.w * (m.r1.x * a1223 - m.r1.y * a0223 + m.r1.z * a0123);
    let det = 1.0 / det;

    Mat4x4 {
        r0: Vec4 {
            x: det * (m.r1.y * a2323 - m.r1.z * a1323 + m.r1.w * a1223),
            y: det * -(m.r0.y * a2323 - m.r0.z * a1323 + m.r0.w * a1223),
            z: det * (m.r0.y * a2313 - m.r0.z * a1313 + m.r0.w * a1213),
            w: det * -(m.r0.y * a2312 - m.r0.z * a1312 + m.r0.w * a1212),
        },
        r1: Vec4 {
            x: det * -(m.r1.x * a2323 - m.r1.z * a0323 + m.r1.w * a0223),
            y: det * (m.r0.x * a2323 - m.r0.z * a0323 + m.r0.w * a0223),
            z: det * -(m.r0.x * a2313 - m.r0.z * a0313 + m.r0.w * a0213),
            w: det * (m.r0.x * a2312 - m.r0.z * a0312 + m.r0.w * a0212),
        },
        r2: Vec4 {
            x: det * (m.r1.x * a1323 - m.r1.y * a0323 + m.r1.w * a0123),
            y: det * -(m.r0.x * a1323 - m.r0.y * a0323 + m.r0.w * a0123),
            z: det * (m.r0.x * a1313 - m.r0.y * a0313 + m.r0.w * a0113),
            w: det * -(m.r0.x * a1312 - m.r0.y * a0312 + m.r0.w * a0112),
        },
        r3: Vec4 {
            x: det * -(m.r1.x * a1223 - m.r1.y * a0223 + m.r1.z * a0123),
            y: det * (m.r0.x * a1223 - m.r0.y * a0223 + m.r0.z * a0123),
            z: det * -(m.r0.x * a1213 - m.r0.y * a0213 + m.r0.z * a0113),
            w: det * (m.r0.x * a1212 - m.r0.y * a0212 + m.r0.z * a0112),
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
    let a = -znear / (zfar - znear);
    let b = (znear * zfar) / (zfar - znear);

    Mat4x4 {
        r0: Vec4 {
            x: w,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: -h,
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

pub fn scale(v: Vec3) -> Mat4x4 {
    Mat4x4 {
        r0: Vec4 {
            x: v.x,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        r1: Vec4 {
            x: 0.0,
            y: v.y,
            z: 0.0,
            w: 0.0,
        },
        r2: Vec4 {
            x: 0.0,
            y: 0.0,
            z: v.z,
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
