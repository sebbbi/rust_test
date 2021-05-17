const SDF_LEVELS: u32 = 6;

use std::env;
use std::process;

use rust_test::sdf;
use rust_test::serialization;
//use rust_test::sparse_sdf;

use sdf::*;
use serialization::*;
//use sparse_sdf::*;

pub struct SdfLevel {
    pub sdf: Sdf,
    pub offset: u32,
}

fn is_correct_size(v: u32, tile_size_payload: u32, padding: u32) -> bool {
    let v_no_pad = v - padding;
    (v_no_pad % tile_size_payload) == 0
}

pub struct Params {
    pub file_in: String,
    pub file_out: String,
}

fn parse_args(args: &[String]) -> Result<Params, &str> {
    if args.len() < 3 {
        return Err("Not enough arguments");
    }

    let file_in = args[1].clone();
    let file_out = args[2].clone();
    Ok(Params { file_in, file_out })
}

fn print_usage() {
    println!("Usage: sfd2tilemap input.sdf output.map");
    println!("(TODO)Tile size: -t [size] (outer size, default 8)");
    println!("(TODO)Levels: -l [levels] (mip levels, default 6)");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let params = parse_args(&args).unwrap_or_else(|err| {
        println!("Argument error: {}", err);
        print_usage();
        process::exit(1);
    });

    println!("Load SDF {}", params.file_in);
    let sdf = load_sdf_zlib(&params.file_in).expect("SDF loading failed");

    let tile_size_payload = 7;
    let tile_size_outer = 8;

    let padding = 1 << SDF_LEVELS;

    // Check size
    // - Must be dividable by: tile_size_payload + 2^SFD_LEVELS
    // - This way the lowest mip level still has 1 pixel filtering border
    let correct_size = is_correct_size(sdf.header.dim.0, tile_size_payload, padding)
        & is_correct_size(sdf.header.dim.1, tile_size_payload, padding)
        & is_correct_size(sdf.header.dim.2, tile_size_payload, padding);

    if !correct_size {
        println!(
            "ERROR: SDF volume size must be dividable with {} + padding {}",
            tile_size_payload, padding
        );
        return;
    }

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
    println!(
        "Finding edge tiles. Tile size, payload = {}x{}x{}, outer = {}x{}x{}",
        tile_size_payload,
        tile_size_payload,
        tile_size_payload,
        tile_size_outer,
        tile_size_outer,
        tile_size_outer
    );

    let mut storer_header = StorerVec::new();
    let mut storer_voxels = StorerVec::new();
    let mut total_tile_count = 0;

    for (i, level) in sdf_levels.iter().enumerate() {
        let dim = level.sdf.header.dim;

        let stride_y = dim.0;
        let stride_z = dim.0 * dim.1;
        let level_zero = (65536 / 2) as u16;
        let mut mip_tile_count = 0;
        let mut edge_tile_count = 0;

        for z in 0..(dim.2 / tile_size_payload) {
            for y in 0..(dim.1 / tile_size_payload) {
                for x in 0..(dim.0 / tile_size_payload) {
                    // Test edge: contains both positive and negative voxels
                    let tile_offset = tile_size_payload * (z * stride_z + y * stride_y + x);
                    let mut has_inside = false;
                    let mut has_outside = false;
                    for iz in 0..tile_size_outer {
                        for iy in 0..tile_size_outer {
                            for ix in 0..tile_size_outer {
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

                    // Edge tile?
                    if has_inside && has_outside {
                        edge_tile_count += 1;

                        // Store voxels
                        for iz in 0..tile_size_outer {
                            for iy in 0..tile_size_outer {
                                for ix in 0..tile_size_outer {
                                    let voxel_offset = iz * stride_z + iy * stride_y + ix;
                                    let d = level.sdf.voxels
                                        [tile_offset as usize + voxel_offset as usize];
                                    storer_voxels.store_u16(d);
                                }
                            }
                        }
                    }
                    mip_tile_count += 1;
                }
            }
        }

        println!(
            "Level = {}: Tiles = {}, Edge tiles = {} ({}%)",
            i,
            mip_tile_count,
            edge_tile_count,
            edge_tile_count as f32 * 100.0 / mip_tile_count as f32
        );

        storer_header.store_u32(edge_tile_count);
        total_tile_count += edge_tile_count;
    }

    let mut storer = StorerVec::new();
    storer.store_array_u8(&storer_header.v);
    storer.store_array_u8(&storer_voxels.v);

    println!(
        "Storing tiles = {}, bytes = {} to {}",
        total_tile_count,
        storer.v.len(),
        params.file_out
    );

    std::fs::write(params.file_out, storer.v).expect("Tilemap store failed");
}
