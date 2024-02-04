use std::fs::File;
use std::io::{BufReader, BufWriter};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::Parser;
const DEFAULT_SIZE: usize = 2048;

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,

    #[clap(short, long, default_value_t = DEFAULT_SIZE)]
    xsize: usize,
    #[clap(short, long, default_value_t = DEFAULT_SIZE)]
    ysize: usize,
}

fn main() {
    let args = Cli::parse();
    let nx = args.xsize;
    let ny = args.ysize;

    let file = File::open(&args.path).expect("failed to open file");
    let mut reader = BufReader::new(&file);

    let n_image = file.metadata().unwrap().len() as usize / nx / ny / 2;
    print!("n images: {}\n", n_image);

    let sum = mean_images(&mut reader, nx, ny, n_image);
    let out_path = args.path.with_extension("sum");
    save_image(sum, &out_path);
}

fn mean_images(reader: &mut BufReader<&File>, nx: usize, ny: usize, n_image: usize) -> Vec<f64> {
    let mut buffer = vec![0u16; nx];
    let mut sum = vec![0f64; nx * ny];

    for i in 0..n_image {
        for y in 0..ny {
            reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();

            let shift_x: i32 = i as i32 % 50;
            let shift_y: i32 = 0;
            let new_y = y as i32 + shift_y;

            if new_y >= ny as i32 || new_y < 0 {
                continue;
            }

            for x in 0..nx {
                let new_x = x as i32 + shift_x;
                if new_x >= nx as i32 || new_x < 0 {
                    continue;
                }
                sum[new_y as usize * nx + new_x as usize] += buffer[x] as f64;
            }
        }
    }

    for i in 0..nx * ny {
        sum[i] /= n_image as f64;
    }

    sum
}

fn save_image(sum: Vec<f64>, path: &std::path::Path) {
    let out_file = File::create(&path).expect("failed to create file");
    let mut writer = BufWriter::new(&out_file);

    for value in sum {
        let as_u16 = value as u16;
        writer.write_u16::<LittleEndian>(as_u16).unwrap();
    }
}
