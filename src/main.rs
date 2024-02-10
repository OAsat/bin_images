use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use clap::{Args, Parser, Subcommand};
const DEFAULT_SIZE: usize = 2048;

#[derive(Parser)]
struct Cli {
    path: PathBuf,
    #[clap(short, long, default_value_t = DEFAULT_SIZE)]
    size: usize,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Detect(DetectArgs),
    Mean(MeanArgs),
    Select(SelectArgs),
    Analyze(AnalyzeArgs),
}

#[derive(Args)]
struct MeanArgs {
    #[clap(short, long)]
    drift_data: Option<PathBuf>,
    #[clap(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct DetectArgs {
    #[clap(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct SelectArgs {
    index: usize,
    #[clap(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct AnalyzeArgs {
    index: usize,
    #[clap(short, long)]
    output: Option<PathBuf>,
}

struct Images {
    file: File,
    size: usize,
    n_image: usize,
}

impl Images {
    fn new(path: &PathBuf, size: usize) -> Images {
        let file = File::open(path).expect("failed to open file");
        let n_image = file.metadata().unwrap().len() as usize / size / size / 2;
        print!("n images: {}\n", n_image);
        Images {
            file,
            size,
            n_image,
        }
    }

    fn get_reader(&self) -> BufReader<&File> {
        BufReader::new(&self.file)
    }

    fn get_reader_from(&self, index: usize) -> BufReader<&File> {
        let mut reader = self.get_reader();
        let pos = (index * self.sq_size() * 2) as u64;
        reader.seek(SeekFrom::Start(pos)).unwrap();
        reader
    }

    fn sq_size(&self) -> usize {
        self.size * self.size
    }
}

fn main() {
    let args = Cli::parse();
    let path = args.path.clone();
    let images = Images::new(&path, args.size);

    match args.command {
        Command::Detect(sub_args) => detect(&images, path, sub_args),
        Command::Mean(sub_args) => mean(&images, path, sub_args),
        Command::Select(sub_args) => select(&images, path, sub_args),
        Command::Analyze(sub_args) => analyze(&images, path, sub_args),
    }
}

fn detect(images: &Images, path: PathBuf, subargs: DetectArgs) {
    let out_path = subargs
        .output
        .unwrap_or_else(|| path.with_extension("drift"));

    let mut buffer = vec![0u16; images.sq_size()];
    let mut reader = images.get_reader();
    reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();
    let pos0 = find_point(&buffer, images.size);

    let mut out: Vec<i16> = Vec::with_capacity(images.n_image * 3);

    for i in 1..images.n_image {
        reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();
        let pos = find_point(&buffer, images.size);
        let x_drift = pos[0] as i16 - pos0[0] as i16;
        let y_drift = pos[1] as i16 - pos0[1] as i16;
        print!("{:?}, {:?}\n", x_drift, y_drift);
        if x_drift.abs() < 100 && y_drift.abs() < 100 {
            out.extend([i as i16, x_drift, y_drift]);
        }
    }

    write_drift_data(&out_path, &out);
}

fn read_drift_data(path: &std::path::Path) -> Vec<i16> {
    let file = File::open(&path).expect("failed to open file");
    let mut reader = BufReader::new(&file);
    let mut buffer = vec![0i16; 3];
    let mut out = Vec::new();

    while let Ok(_) = reader.read_i16_into::<LittleEndian>(&mut buffer) {
        out.extend(&buffer);
    }
    out
}

fn write_drift_data(path: &std::path::Path, drift_data: &[i16]) {
    let out_file = File::create(&path).expect("failed to create file");
    let mut writer = BufWriter::new(&out_file);

    for value in drift_data {
        writer.write_i16::<LittleEndian>(*value).unwrap();
    }
}

fn select(images: &Images, path: PathBuf, subargs: SelectArgs) {
    let index = subargs.index;
    let out_path = subargs
        .output
        .unwrap_or_else(|| path.with_extension(format!("{}.single", index).as_str()));

    let mut image = vec![0u16; images.sq_size()];
    let mut reader = images.get_reader_from(index);
    reader
        .read_u16_into::<LittleEndian>(&mut image)
        .expect("index seems to be out of range");
    save_u16image(&image, &out_path);
}

fn analyze(images: &Images, path: PathBuf, subargs: AnalyzeArgs) {
    let index = subargs.index;
    let out_path = subargs
        .output
        .unwrap_or_else(|| path.with_extension(format!("{}.analyzed", index).as_str()));

    let mut image = vec![0u16; images.sq_size()];
    let mut reader = images.get_reader_from(index);
    reader.read_u16_into::<LittleEndian>(&mut image).unwrap();
    let result = analyze_single(&image);
    save_u16image(&result, &out_path);
}

fn analyze_single(image: &[u16]) -> Vec<u16> {
    let new_size = 512;
    let rate = 0.01;
    let mut small = shrink_image(&image, new_size);
    small.sort();
    let thresh = small[small.len() * (1.0 - rate) as usize];

    let mut result = vec![0u16; new_size * new_size];
    for value in result.iter_mut() {
        if *value < thresh {
            *value = 0;
        } else {
            *value = 1;
        }
    }
    result
}

fn shrink_image(image: &[u16], new_size: usize) -> Vec<u16> {
    let original_size = (image.len() as f64).sqrt() as usize;
    let mut new_image = vec![0u16; new_size * new_size];
    let unit = original_size / new_size;

    for (new_line, original_block) in new_image
        .chunks_exact_mut(new_size)
        .zip(image.chunks_exact(original_size * unit))
    {
        let mut temp_line = vec![0f64; new_size];
        for original_line in original_block.chunks_exact(original_size) {
            for (i, chunk) in original_line.chunks_exact(unit).enumerate() {
                temp_line[i] += chunk.iter().fold(0f64, |acc, &x| acc + x as f64);
            }
        }
        for (i, value) in temp_line.iter().enumerate() {
            new_line[i] = (*value / (unit as f64).powf(2.0)) as u16;
        }
    }
    new_image
}

fn mean(images: &Images, path: PathBuf, subargs: MeanArgs) {
    let out_path = subargs
        .output
        .unwrap_or_else(|| path.with_extension("mean"));

    match subargs.drift_data {
        Some(drift_path) => {
            let drift_data = read_drift_data(&drift_path);
            let sum = calc_shifted_mean(&images, &drift_data);
            save_f64image(&sum, &out_path);
        }
        None => {
            let sum = simple_mean(&images, images.n_image);
            save_f64image(&sum, &out_path);
        }
    }
}

fn simple_mean(images: &Images, take_first_n: usize) -> Vec<f64> {
    let mut buffer = vec![0u16; images.sq_size()];
    let mut sum = vec![0f64; images.sq_size()];
    let mut reader = images.get_reader();
    let n = std::cmp::min(take_first_n, images.n_image);

    for _ in 0..n {
        reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();
        for (value, sum_value) in buffer.iter().zip(sum.iter_mut()) {
            *sum_value += *value as f64;
        }
    }

    for value in sum.iter_mut() {
        *value /= n as f64;
    }
    sum
}

fn calc_shifted_mean(images: &Images, drift_data: &[i16]) -> Vec<f64> {
    let size = images.size;
    let mut buffer = vec![0u16; size * size];
    let mut sum = vec![0f64; size * size];
    let mut count = 0usize;
    let mut reader = images.get_reader();

    for i in 0..images.n_image {
        reader.read_u16_into::<LittleEndian>(&mut buffer).unwrap();
        if drift_data[count * 3] != i as i16 {
            continue;
        }

        let drift_x: i16 = drift_data[count * 3 + 1];
        let drift_y: i16 = drift_data[count * 3 + 2];

        for (y, line) in buffer.chunks_exact(size).enumerate() {
            let new_y = y as i16 + drift_y;

            if new_y >= size as i16 || new_y < 0 {
                continue;
            }

            for (x, value) in line.iter().enumerate() {
                let new_x = x as i16 + drift_x;
                if new_x >= size as i16 || new_x < 0 {
                    continue;
                }
                sum[new_y as usize * size + new_x as usize] += *value as f64;
            }
        }
        count += 1;
    }

    for i in 0..size * size {
        sum[i] /= count as f64;
    }
    print!("{} images averaged.\n", count);

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

fn find_point(image: &[u16], size: usize) -> [usize; 2] {
    let mut max = 0;
    let mut max_x = 0;
    let mut max_y = 0;

    for i in 0..size * size {
        let value = image[i];
        if value > max {
            max = value;
            max_x = i % size;
            max_y = i / size;
        }
    }

    [max_x, max_y]
}
