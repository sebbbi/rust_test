use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::Read;

#[derive(Clone, Debug, Copy)]
struct SdfHeader {
    i: i32,
    j: i32,
    k: i32,
    box_min: [f32; 3],
    dx: f32,
}

struct Loader {
    offset: usize,
}

impl Loader {
    pub fn new() -> Loader {
        Loader { offset: 0 }
    }

    pub fn load_i32(&mut self, bytes: &[u8]) -> i32 {
        let out = i32::from_le_bytes(bytes[self.offset..self.offset + 4].try_into().unwrap());
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

pub fn load_sdf(filename: &str) -> io::Result<()> {
    let bytes = std::fs::read(filename)?;

    let mut loader = Loader::new();
    let header = SdfHeader {
        i: loader.load_i32(&bytes),
        j: loader.load_i32(&bytes),
        k: loader.load_i32(&bytes),
        box_min: [
            loader.load_f32(&bytes),
            loader.load_f32(&bytes),
            loader.load_f32(&bytes),
        ],
        dx: loader.load_f32(&bytes),
    };

    let count_voxels = header.i * header.j * header.k;
    let voxels = loader.load_array_f32(&bytes, count_voxels as usize);

    println!("Header {:?}", header);

    println!("Voxels {:?}", voxels[0]);

    Ok(())
}
