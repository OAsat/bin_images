use std::fs::File;
use std::io::{BufReader, BufWriter};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::{Parser, ValueEnum};
const DEFAULT_SIZE: usize = 2048;

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,

    #[clap(short, long, default_value = "mean")]
    mode: Mode,

    #[clap(short, long, default_value_t = DEFAULT_SIZE)]
    xsize: usize,
    #[clap(short, long, default_value_t = DEFAULT_SIZE)]
    ysize: usize,

    #[clap(short, long)]
    shift_data: Option<std::path::PathBuf>,
}

#[derive(ValueEnum, Clone)]
enum Mode {
    Detect,
    Mean,
    All,
}

fn main() {
    let args = Cli::parse();

    match args.mode {
        Mode::Mean => mean_mode(args),
        Mode::Detect => detect_mode(args),
        Mode::All => all_mode(args),
    }
}

fn detect_mode(args: Cli) {
    let nx = args.xsize;
    let ny = args.ysize;

    let file = File::open(&args.path).expect("failed to open file");

    let n_image = file.metadata().unwrap().len() as usize / nx / ny / 2;
    print!("n images: {}\n", n_image);

    let mut buffer = vec![0u16; nx * ny];
    let mut reader = BufReader::new(&file);

    reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();
    let pos0 = find_point(&buffer, nx, ny);
    print!("pos0: {:?}\n", pos0);

    let mut out = vec![0i16; n_image * 3];

    for i in 1..n_image {
        reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();
        let pos = find_point(&buffer, nx, ny);
        out[3 * i] = i as i16;
        out[3 * i + 1] = pos[0] as i16 - pos0[0] as i16;
        out[3 * i + 2] = pos[1] as i16 - pos0[1] as i16;
    }

    let out_path = args.path.with_extension("shift");
    let out_file = File::create(&out_path).expect("failed to create file");
    let mut writer = BufWriter::new(&out_file);

    for value in out {
        writer.write_i16::<LittleEndian>(value).unwrap();
    }
}

fn find_point(image: &[u16], nx: usize, ny: usize) -> [usize; 2] {
    let mut max = 0;
    let mut max_x = 0;
    let mut max_y = 0;

    for i in 0..nx * ny {
        let value = image[i];
        if value > max {
            max = value;
            max_x = i % nx;
            max_y = i / ny;
        }
    }

    [max_x, max_y]
}

fn all_mode(_args: Cli) {
    unimplemented!()
}

fn mean_mode(args: Cli) {
    let nx = args.xsize;
    let ny = args.ysize;

    let file = File::open(&args.path).expect("failed to open file");

    let n_image = file.metadata().unwrap().len() as usize / nx / ny / 2;
    print!("n images: {}\n", n_image);

    let shift_data = if let Some(shift_path) = args.shift_data {
        let mut shift_file = File::open(&shift_path).expect("failed to open file");
        let mut buffer = vec![0i16; n_image * 3];
        shift_file
            .read_i16_into::<LittleEndian>(&mut buffer)
            .unwrap();
        buffer
    } else {
        vec![0i16; n_image * 3]
    };

    let mut reader = BufReader::new(&file);
    let sum = mean_images(&mut reader, nx, ny, n_image, &shift_data);

    let out_path = args.path.with_extension("sum");
    save_image(sum, &out_path);
}

fn mean_images(
    reader: &mut BufReader<&File>,
    nx: usize,
    ny: usize,
    n_image: usize,
    shift_data: &[i16],
) -> Vec<f64> {
    let mut buffer = vec![0u16; nx];
    let mut sum = vec![0f64; nx * ny];

    for i in 0..n_image {
        for y in 0..ny {
            reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();

            let shift_x: i16 = shift_data[i * 3 + 1];
            let shift_y: i16 = shift_data[i * 3 + 2];
            let new_y = y as i16 + shift_y;

            if new_y >= ny as i16 || new_y < 0 {
                continue;
            }

            for x in 0..nx {
                let new_x = x as i16 + shift_x;
                if new_x >= nx as i16 || new_x < 0 {
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
