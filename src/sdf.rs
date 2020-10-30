extern crate miniz_oxide;
use miniz_oxide::deflate::compress_to_vec;
use miniz_oxide::inflate::decompress_to_vec;

use crate::serialization::*;

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
    pub voxels: Vec<u16>,
}

pub fn load_sdf_zlib(filename: &str) -> io::Result<Sdf> {
    let bytes = std::fs::read(filename)?;
    let bytes = decompress_to_vec(&bytes[..]).expect("Failed to decompress!");

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
    let voxels = loader.load_array_u16(&bytes, count_voxels as usize);

    println!("Loaded SDF: {:?}", header);

    let sdf = Sdf { header, voxels };
    let sdf = decompress_postprocess_sdf(&sdf);

    Ok(sdf)
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
    let voxels = loader.load_array_u16(&bytes, count_voxels as usize);

    println!("Loaded SDF: {:?}", header);

    let sdf = Sdf { header, voxels };

    Ok(sdf)
}

pub fn store_sdf_zlib(filename: &str, sdf: &Sdf) -> io::Result<()> {
    let sdf = compress_preprocess_sdf(&sdf);

    let byte_count =
        sdf.voxels.len() as usize * std::mem::size_of::<u16>() + std::mem::size_of::<SdfHeader>();
    let mut bytes = vec![0u8; byte_count];

    let mut storer = Storer::new();

    storer.store_u32(&mut bytes[..], sdf.header.dim.0);
    storer.store_u32(&mut bytes[..], sdf.header.dim.1);
    storer.store_u32(&mut bytes[..], sdf.header.dim.2);
    storer.store_f32(&mut bytes[..], sdf.header.box_min.0);
    storer.store_f32(&mut bytes[..], sdf.header.box_min.1);
    storer.store_f32(&mut bytes[..], sdf.header.box_min.2);
    storer.store_f32(&mut bytes[..], sdf.header.dx);

    storer.store_array_u16(&mut bytes[..], &sdf.voxels[..]);

    let bytes = compress_to_vec(&bytes[..], 6);
    std::fs::write(filename, bytes)?;

    Ok(())
}

pub fn store_sdf(filename: &str, sdf: &Sdf) -> io::Result<()> {
    let byte_count =
        sdf.voxels.len() as usize * std::mem::size_of::<u16>() + std::mem::size_of::<SdfHeader>();
    let mut bytes = vec![0u8; byte_count];

    let mut storer = Storer::new();

    storer.store_u32(&mut bytes[..], sdf.header.dim.0);
    storer.store_u32(&mut bytes[..], sdf.header.dim.1);
    storer.store_u32(&mut bytes[..], sdf.header.dim.2);
    storer.store_f32(&mut bytes[..], sdf.header.box_min.0);
    storer.store_f32(&mut bytes[..], sdf.header.box_min.1);
    storer.store_f32(&mut bytes[..], sdf.header.box_min.2);
    storer.store_f32(&mut bytes[..], sdf.header.dx);

    storer.store_array_u16(&mut bytes[..], &sdf.voxels[..]);

    std::fs::write(filename, bytes)?;

    Ok(())
}

pub enum AxisFlip {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

pub fn orient_sdf(sdf: &Sdf, x_orient: AxisFlip, y_orient: AxisFlip, z_orient: AxisFlip) -> Sdf {
    let stride_x = 1i32;
    let stride_y = (sdf.header.dim.0) as i32;
    let stride_z = (sdf.header.dim.0 * sdf.header.dim.1) as i32;

    let orientation = |orient| match orient {
        AxisFlip::PositiveX => (sdf.header.dim.0 as i32, 0, 1, stride_x),
        AxisFlip::NegativeX => (
            sdf.header.dim.0 as i32,
            sdf.header.dim.0 as i32 - 1,
            -1,
            stride_x,
        ),
        AxisFlip::PositiveY => (sdf.header.dim.1 as i32, 0, 1, stride_y),
        AxisFlip::NegativeY => (
            sdf.header.dim.1 as i32,
            sdf.header.dim.1 as i32 - 1,
            -1,
            stride_y,
        ),
        AxisFlip::PositiveZ => (sdf.header.dim.2 as i32, 0, 1, stride_z),
        AxisFlip::NegativeZ => (
            sdf.header.dim.2 as i32,
            sdf.header.dim.2 as i32 - 1,
            -1,
            stride_z,
        ),
    };

    let (x_dim, x_start, x_step, x_stride): (i32, i32, i32, i32) = orientation(x_orient);
    let (y_dim, y_start, y_step, y_stride) = orientation(y_orient);
    let (z_dim, z_start, z_step, z_stride) = orientation(z_orient);

    let stride_y = x_dim;
    let stride_z = x_dim * y_dim;

    let mut voxels = vec![0; sdf.voxels.len()];
    for z in 0..z_dim {
        for y in 0..y_dim {
            for x in 0..x_dim {
                let write_addr = x + y * stride_y + z * stride_z;
                let read_addr = (x * x_step + x_start) * x_stride
                    + (y * y_step + y_start) * y_stride
                    + (z * z_step + z_start) * z_stride;
                voxels[write_addr as usize] = sdf.voxels[read_addr as usize];
            }
        }
    }

    let header = SdfHeader {
        dim: (x_dim as u32, y_dim as u32, z_dim as u32),
        box_min: (
            0.0, 0.0, 0.0, // Not used
        ),
        dx: sdf.header.dx,
    };

    Sdf { header, voxels }
}

pub fn downsample_2x_sdf(sdf: &Sdf) -> Sdf {
    let x_dim = sdf.header.dim.0 / 2;
    let y_dim = sdf.header.dim.1 / 2;
    let z_dim = sdf.header.dim.2 / 2;

    let stride_y = (sdf.header.dim.0) as u32;
    let stride_z = (sdf.header.dim.0 * sdf.header.dim.1) as u32;

    let stride_write_y = x_dim as u32;
    let stride_write_z = (x_dim * y_dim) as u32;

    let mut voxels = vec![0; (x_dim * y_dim * z_dim) as usize];
    for z in 0..z_dim {
        for y in 0..y_dim {
            for x in 0..x_dim {
                let write_addr = x + y * stride_write_y + z * stride_write_z;
                let read_addr_base = x * 2 + y * stride_y * 2 + z * stride_z * 2;

                let sum = sdf.voxels[(read_addr_base) as usize] as u32
                    + sdf.voxels[(read_addr_base + 1) as usize] as u32
                    + sdf.voxels[(read_addr_base + stride_y) as usize] as u32
                    + sdf.voxels[(read_addr_base + 1 + stride_y) as usize] as u32
                    + sdf.voxels[(read_addr_base + stride_z) as usize] as u32
                    + sdf.voxels[(read_addr_base + 1 + stride_z) as usize] as u32
                    + sdf.voxels[(read_addr_base + stride_y + stride_z) as usize] as u32
                    + sdf.voxels[(read_addr_base + 1 + stride_y + stride_z) as usize] as u32;

                voxels[write_addr as usize] = (sum / 8) as u16;
            }
        }
    }

    let header = SdfHeader {
        dim: (x_dim as u32, y_dim as u32, z_dim as u32),
        box_min: (
            0.0, 0.0, 0.0, // Not used
        ),
        dx: sdf.header.dx * 2.0,
    };

    Sdf { header, voxels }
}

// https://gist.github.com/mfuerstenau/ba870a29e16536fdbaba
pub fn abs_diff(v: i32) -> u32 {
    // Positive: 0,2,4,6...
    // Negative: 1,3,5,7...
    ((v >> 31) ^ (v << 1)) as u32
}

pub fn abs_diff_inv(v: u32) -> i32 {
    // 0,-1,1,-2,2,-3,3...
    (v >> 1) as i32 ^ -((v & 1) as i32)
}

pub fn compress_preprocess_sdf(sdf: &Sdf) -> Sdf {
    let x_dim = sdf.header.dim.0 as usize;
    let y_dim = sdf.header.dim.1 as usize;
    let z_dim = sdf.header.dim.2 as usize;

    let stride_y = (sdf.header.dim.0) as usize;
    let stride_z = (sdf.header.dim.0 * sdf.header.dim.1) as usize;

    let mut voxels = sdf.voxels.clone();

    // NOTE: Storing x=0, y=0, z=0 slices as is
    // TODO: 1d gradient estimate for the first y scanline
    // TODO: 2d gradient estimate for the first z slice

    for z in 1..z_dim {
        for y in 1..y_dim {
            for x in 1..x_dim {
                let addr_base = x + y * stride_y + z * stride_z;

                let dx = sdf.voxels[addr_base - stride_y] as i32
                    - sdf.voxels[addr_base - stride_y - 1] as i32;

                let dy =
                    sdf.voxels[addr_base - 1] as i32 - sdf.voxels[addr_base - stride_y - 1] as i32;

                let dz =
                    sdf.voxels[addr_base - 1] as i32 - sdf.voxels[addr_base - stride_z - 1] as i32;

                // TODO: Use eikonal equation instead of this simple linear estimate
                let estimate = sdf.voxels[addr_base - 1] as i32 + dx;

                let v = sdf.voxels[addr_base] as i32;
                voxels[addr_base] = abs_diff(v - estimate) as u16;
            }
        }
    }

    let header = SdfHeader {
        dim: (x_dim as u32, y_dim as u32, z_dim as u32),
        box_min: (
            0.0, 0.0, 0.0, // Not used
        ),
        dx: sdf.header.dx,
    };

    Sdf { header, voxels }
}

pub fn decompress_postprocess_sdf(sdf: &Sdf) -> Sdf {
    let x_dim = sdf.header.dim.0 as usize;
    let y_dim = sdf.header.dim.1 as usize;
    let z_dim = sdf.header.dim.2 as usize;

    let stride_y = (sdf.header.dim.0) as usize;
    let stride_z = (sdf.header.dim.0 * sdf.header.dim.1) as usize;

    let mut voxels = sdf.voxels.clone();

    // NOTE: Storing x=0, y=0, z=0 slices as is
    // TODO: 1d gradient estimate for the first y scanline
    // TODO: 2d gradient estimate for the first z slice

    for z in 1..z_dim {
        for y in 1..y_dim {
            for x in 1..x_dim {
                let addr_base = x + y * stride_y + z * stride_z;

                let dx =
                    voxels[addr_base - stride_y] as i32 - voxels[addr_base - stride_y - 1] as i32;

                let dy = voxels[addr_base - 1] as i32 - voxels[addr_base - stride_y - 1] as i32;

                let dz = voxels[addr_base - 1] as i32 - voxels[addr_base - stride_z - 1] as i32;

                // TODO: Use eikonal equation instead of this simple linear estimate
                let estimate = voxels[addr_base - 1] as i32 + dx;

                let v = voxels[addr_base] as u32;
                voxels[addr_base] = (estimate + abs_diff_inv(v)) as u16;
            }
        }
    }

    let header = SdfHeader {
        dim: (x_dim as u32, y_dim as u32, z_dim as u32),
        box_min: (
            0.0, 0.0, 0.0, // Not used
        ),
        dx: sdf.header.dx,
    };

    Sdf { header, voxels }
}
