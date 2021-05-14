#![allow(dead_code)]

const SDF_LEVELS: u32 = 6;

use std::env;
//use rust_test::minivector;
//use rust_test::serialization;
use rust_test::sdf;
//use rust_test::sparse_sdf;

//use minivector::*;
use sdf::*;
//use sparse_sdf::*;

pub struct SdfLevel {
    pub sdf: Sdf,
    pub offset: u32,
}

fn main() {
    // Distance field
    let sdf = load_sdf_zlib("data/ganymede-and-jupiter.sdf").expect("SDF loading failed");

    // Generate mips
    let mut sdf_levels = Vec::new();
    let mut sdf_total_voxels = sdf.header.dim.0 * sdf.header.dim.1 * sdf.header.dim.2;
    sdf_levels.push(SdfLevel { sdf, offset: 0 });
    for _ in 1..SDF_LEVELS {
        let sdf = downsample_2x_sdf(&sdf_levels.last().unwrap().sdf);
        let offset = sdf_total_voxels;
        sdf_total_voxels += sdf.header.dim.0 * sdf.header.dim.1 * sdf.header.dim.2;
        sdf_levels.push(SdfLevel { sdf, offset });
    }

    // Find all edge tiles
    let tile_size = 8;
    println!(
        "Finding edge tiles. Tile size = {}x{}x{}",
        tile_size, tile_size, tile_size
    );
    for (i, level) in sdf_levels.iter().enumerate() {
        let dim = level.sdf.header.dim;

        let stride_y = dim.0;
        let stride_z = dim.0 * dim.1;
        let level_zero = (65536 / 2) as u16;
        let mut total_tile_count = 0;
        let mut edge_tile_count = 0;

        for z in 0..(dim.2 / tile_size) {
            for y in 0..(dim.1 / tile_size) {
                for x in 0..(dim.0 / tile_size) {
                    let tile_offset = tile_size * (z * stride_z + y * stride_y + x);
                    let mut has_inside = false;
                    let mut has_outside = false;
                    for iz in 0..tile_size {
                        for iy in 0..tile_size {
                            for ix in 0..tile_size {
                                let voxel_offset = iz * stride_z + iy * stride_y + ix;
                                let d =
                                    level.sdf.voxels[tile_offset as usize + voxel_offset as usize];
                                if d < level_zero {
                                    has_inside = true;
                                };
                                if d > level_zero {
                                    has_outside = true;
                                };
                            }
                        }
                    }
                    if has_inside && has_outside {
                        edge_tile_count += 1;
                    }
                    total_tile_count += 1;
                }
            }
        }

        println!(
            "Level = {}: Total tiles = {}, Edge tiles = {} ({}%)",
            i,
            total_tile_count,
            edge_tile_count,
            edge_tile_count as f32 * 100.0 / total_tile_count as f32
        );
    }

    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
}
