use std::env;
use std::process;

use rust_test::sdf;

use sdf::*;

pub struct SdfLevel {
    pub sdf: Sdf,
    pub offset: u32,
}

pub struct Params {
    pub file_in: String,
    pub file_out: String,
    pub axis_x: AxisFlip,
    pub axis_y: AxisFlip,
    pub axis_z: AxisFlip,
    pub compressed_input: bool,
    pub compressed_output: bool,
}

fn parse_args(args: &[String]) -> Result<Params, &str> {
    if args.len() < 3 {
        return Err("Not enough arguments");
    }

    let file_in = args[1].clone();
    let file_out = args[2].clone();

    // TODO: FIXME! Hard coded axis flip!
    let axis_x = AxisFlip::PositiveX;
    let axis_y = AxisFlip::PositiveZ;
    let axis_z = AxisFlip::PositiveY;

    let mut compressed_input = false;
    let mut compressed_output = false;

    for arg in args.iter().skip(3) {
        match &arg[..] {
            "-iz" => compressed_input = true,
            "-oz" => compressed_output = true,
            _ => (),
        }
    }

    Ok(Params {
        file_in,
        file_out,
        axis_x,
        axis_y,
        axis_z,
        compressed_input,
        compressed_output,
    })
}

fn print_usage() {
    println!("Usage: sdftool input.sdf output.sdf args");
    println!("Compressed input (grad+zlib): -iz");
    println!("Compress output (grad+zlib): -oz");
    println!("(TODO) Orient/flip axis: -o xZy (xyz = axis, capital letter = negate)");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let params = parse_args(&args).unwrap_or_else(|err| {
        println!("Argument error: {}", err);
        print_usage();
        process::exit(1);
    });

    let sdf = if params.compressed_input {
        println!("Load SDF with zlib: {}", params.file_in);
        load_sdf_zlib(&params.file_in)
    } else {
        println!("Load SDF: {}", params.file_in);
        load_sdf(&params.file_in)
    }
    .expect("SDF loading failed");

    println!(
        "Orient SDF x = {:?}, y = {:?}, z = {:?}",
        params.axis_x, params.axis_y, params.axis_z
    );
    let sdf = orient_sdf(&sdf, params.axis_x, params.axis_y, params.axis_z);

    if params.compressed_output {
        println!("Store SDF with zlib: {}", params.file_out);
        store_sdf_zlib(&params.file_out, &sdf)
    } else {
        println!("Store SDF: {}", params.file_out);
        store_sdf(&params.file_out, &sdf)
    }
    .expect("SDF store failed");
}
