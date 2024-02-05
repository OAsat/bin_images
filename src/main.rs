use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::io::{Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::{Args, Parser, Subcommand};
const DEFAULT_SIZE: usize = 2048;

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,

    #[clap(short, long, default_value_t = DEFAULT_SIZE)]
    xsize: usize,
    #[clap(short, long, default_value_t = DEFAULT_SIZE)]
    ysize: usize,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Detect(DetectArgs),
    Mean(MeanArgs),
    Select(SelectArgs),
}

#[derive(Args)]
struct MeanArgs {
    #[clap(short, long)]
    drift_data: Option<std::path::PathBuf>,
    #[clap(short, long)]
    output: Option<std::path::PathBuf>,
}

#[derive(Args)]
struct DetectArgs {
    #[clap(short, long)]
    output: Option<std::path::PathBuf>,
}

#[derive(Args)]
struct SelectArgs {
    index: usize,
    #[clap(short, long)]
    output: Option<std::path::PathBuf>,
}

struct Images {
    path: std::path::PathBuf,
    reader: BufReader<File>,
    nx: usize,
    ny: usize,
    n_image: usize,
}

fn get_images(args: &Cli) -> Images {
    let file = File::open(&args.path).expect("failed to open file");
    let n_image = file.metadata().unwrap().len() as usize / args.xsize / args.ysize / 2;
    print!("n images: {}\n", n_image);
    Images {
        path: args.path.clone(),
        reader: BufReader::new(file),
        nx: args.xsize,
        ny: args.ysize,
        n_image: n_image,
    }
}

fn main() {
    let args = Cli::parse();
    let mut images = get_images(&args);

    match args.command {
        Command::Detect(detect_args) => detect(&mut images, detect_args),
        Command::Mean(mean_args) => mean(&mut images, mean_args),
        Command::Select(select_args) => select(&mut images, select_args),
    }
}

fn detect(images: &mut Images, args: DetectArgs) {
    let nx = images.nx;
    let ny = images.ny;

    let mut buffer = vec![0u16; nx * ny];
    images
        .reader
        .read_u16_into::<LittleEndian>(&mut buffer)
        .unwrap();
    let pos0 = find_point(&buffer, nx, ny);

    let mut out: Vec<i16> = Vec::with_capacity(images.n_image * 3);

    for i in 1..images.n_image {
        images
            .reader
            .read_u16_into::<LittleEndian>(&mut buffer)
            .unwrap();
        let pos = find_point(&buffer, nx, ny);
        let x_drift = pos[0] as i16 - pos0[0] as i16;
        let y_drift = pos[1] as i16 - pos0[1] as i16;
        print!("{:?}, {:?}\n", x_drift, y_drift);
        if x_drift.abs() < 100 && y_drift.abs() < 100 {
            out.extend([i as i16, x_drift, y_drift]);
        }
    }

    let out_path = args
        .output
        .unwrap_or_else(|| images.path.with_extension("drift"));
    let out_file = File::create(&out_path).expect("failed to create file");
    let mut writer = BufWriter::new(&out_file);

    for value in out {
        writer.write_i16::<LittleEndian>(value).unwrap();
    }
}

fn select(images: &mut Images, args: SelectArgs) {
    let mut image = vec![0u16; images.nx * images.ny];
    images
        .reader
        .seek(SeekFrom::Start(
            (args.index * images.nx * images.ny * 2) as u64,
        ))
        .unwrap();
    images
        .reader
        .read_u16_into::<LittleEndian>(&mut image)
        .unwrap();
    let out_path = args
        .output
        .unwrap_or_else(|| images.path.with_extension("selected"));
    save_u16image(&image, &out_path);
}

fn mean(images: &mut Images, args: MeanArgs) {
    let drift_data = if let Some(drift_path) = args.drift_data {
        let mut drift_file = File::open(&drift_path).expect("failed to open file");
        let n_mean = drift_file.metadata().unwrap().len() as usize / 6;
        let mut buffer = vec![0i16; n_mean * 3];
        drift_file
            .read_i16_into::<LittleEndian>(&mut buffer)
            .unwrap();
        buffer
    } else {
        (0..images.n_image as i16)
            .map(|i| [i, 0, 0])
            .flatten()
            .collect()
    };

    let sum = calc_images_mean(images, &drift_data);

    let out_path = args
        .output
        .unwrap_or_else(|| images.path.with_extension("mean"));
    save_f64image(&sum, &out_path);
}

fn calc_images_mean(images: &mut Images, drift_data: &[i16]) -> Vec<f64> {
    let mut buffer = vec![0u16; images.nx * images.ny];
    let mut sum = vec![0f64; images.nx * images.ny];
    let mut count = 0usize;

    for i in 0..images.n_image {
        images
            .reader
            .read_u16_into::<LittleEndian>(&mut buffer)
            .unwrap();
        if drift_data[count * 3] != i as i16 {
            continue;
        }

        let drift_x: i16 = drift_data[count * 3 + 1];
        let drift_y: i16 = drift_data[count * 3 + 2];

        for (y, line) in buffer.chunks_exact(images.nx).enumerate() {
            let new_y = y as i16 + drift_y;

            if new_y >= images.ny as i16 || new_y < 0 {
                continue;
            }

            for (x, value) in line.iter().enumerate() {
                let new_x = x as i16 + drift_x;
                if new_x >= images.nx as i16 || new_x < 0 {
                    continue;
                }
                sum[new_y as usize * images.nx + new_x as usize] += *value as f64;
            }
        }
        count += 1;
    }

    for i in 0..images.nx * images.ny {
        sum[i] /= count as f64;
    }
    print!("Average over {} images calculated.\n", count);

    sum
}

fn save_u16image(image: &[u16], path: &std::path::Path) {
    let out_file = File::create(&path).expect("failed to create file");
    let mut writer = BufWriter::new(&out_file);

    for value in image {
        writer.write_u16::<LittleEndian>(*value).unwrap();
    }
}

fn save_f64image(image: &[f64], path: &std::path::Path) {
    let out_file = File::create(&path).expect("failed to create file");
    let mut writer = BufWriter::new(&out_file);

    for value in image {
        let as_u16 = *value as u16;
        writer.write_u16::<LittleEndian>(as_u16).unwrap();
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
