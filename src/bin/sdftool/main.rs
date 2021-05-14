#![allow(dead_code)]

use std::process;

use rust_test::sdf;
use std::env;

use sdf::*;

pub struct SdfLevel {
    pub sdf: Sdf,
    pub offset: u32,
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
    println!("Usage: sdftool input.sdf output.sdf");
    println!("Orient/flip axis: -o xZy (xyz = axis, capital letter = negate)");
    println!("Compressed input (grad+zlib): -iz");
    println!("Compress output (grad+zlib): -oz");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let params = parse_args(&args).unwrap_or_else(|err| {
        println!("Argument error: {}", err);
        print_usage();
        process::exit(1);
    });

    // TODO: FIXME! Hard coded axis flip only!
    // TODO: FIXME! Hard coded compression modes!
    let axis_x = AxisFlip::PositiveX;
    let axis_y = AxisFlip::PositiveZ;
    let axis_z = AxisFlip::PositiveY;

    println!("Load SDF {}", params.file_in);
    let sdf = load_sdf_zlib(&params.file_in).expect("SDF loading failed");

    println!(
        "Orient SDF x = {:?}, y = {:?}, z = {:?}",
        axis_x, axis_y, axis_z
    );
    let sdf = orient_sdf(&sdf, axis_x, axis_y, axis_z);

    println!("Store SDF with zlib {}", params.file_out);
    store_sdf_zlib(&params.file_out, &sdf).expect("SDF store failed");
}
