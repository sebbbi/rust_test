use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::Read;

#[derive(Clone, Debug, Copy)]
pub struct SdfHeader {
    pub dim: (u32, u32, u32),
    pub box_min: (f32, f32, f32),
    pub dx: f32,
}

pub struct Sdf {
    pub header: SdfHeader,
    pub voxels: Vec<f32>,
}

struct Loader {
    offset: usize,
}

impl Loader {
    pub fn new() -> Loader {
        Loader { offset: 0 }
    }

    pub fn load_u32(&mut self, bytes: &[u8]) -> u32 {
        let out = u32::from_le_bytes(bytes[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        out
    }

    pub fn load_f32(&mut self, bytes: &[u8]) -> f32 {
        let out = f32::from_le_bytes(bytes[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        out
    }

    pub fn load_array_f32(&mut self, bytes: &[u8], count: usize) -> Vec<f32> {
        (0..count)
            .map(|_| {
                let out =
                    f32::from_le_bytes(bytes[self.offset..self.offset + 4].try_into().unwrap());
                self.offset += 4;
                out
            })
            .collect()
    }
}

pub fn load_sdf(filename: &str) -> io::Result<Sdf> {
    let bytes = std::fs::read(filename)?;

    let mut loader = Loader::new();
    let header = SdfHeader {
        dim: (
            loader.load_u32(&bytes),
            loader.load_u32(&bytes),
            loader.load_u32(&bytes),
        ),
        box_min: (
            loader.load_f32(&bytes),
            loader.load_f32(&bytes),
            loader.load_f32(&bytes),
        ),
        dx: loader.load_f32(&bytes),
    };

    let count_voxels = header.dim.0 * header.dim.1 * header.dim.2;
    let voxels = loader.load_array_f32(&bytes, count_voxels as usize);

    println!("Header {:?}", header);
    println!("Voxels {:?}", voxels[0]);

    let sdf = Sdf {
        header,
        voxels,
	};

    Ok(sdf)
}
