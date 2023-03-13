use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{self};
use mozjpeg::{ColorSpace, Compress, ScanMode};
use rayon::prelude::*;

const TARGET_SIZE: usize = 1280;

fn main() {
    let target_dir = match env::args().nth(1) {
        Some(v) => PathBuf::from(v).parent().unwrap().join(""),
        None => return,
    };
    if !target_dir.exists() {
        fs::create_dir(&target_dir).unwrap();
    }

    let source_files = env::args()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_file() && p.file_name().is_some() && p.extension().is_some())
        .collect::<Vec<PathBuf>>();

    source_files
        .par_iter()
        .for_each(|path| match process(&path, &target_dir) {
            Ok(file_name) => println!("{} is compressed.", file_name),
            Err((file_name, err)) => println!("{} FAILED due to {}", file_name, err),
        });
    pause();
}

fn process(path: &Path, target_dir: &Path) -> Result<String, (String, String)> {
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();

    let (resized_img_data, target_width, target_height) = match resize(path) {
        Ok(v) => v,
        Err(e) => return Err((file_name, e.to_string())),
    };

    let compressed_img_data = match compress(resized_img_data, target_width, target_height) {
        Ok(v) => v,
        Err(e) => return Err((file_name, e.to_string())),
    };

    let target_file = target_dir.join("resized_".to_string() + &file_name);
    let mut file =
        BufWriter::new(File::create(target_file).map_err(|e| (file_name.clone(), e.to_string()))?);
    file.write_all(&compressed_img_data)
        .map_err(|e| (file_name.clone(), e.to_string()))?;

    Ok(file_name)
}

fn resize(path: &Path) -> Result<(Vec<u8>, usize, usize), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let width = img.width() as usize;
    let height = img.height() as usize;

    if width > TARGET_SIZE || height > TARGET_SIZE {
        let (target_width, target_height) = if width > height {
            let ratio: f32 = TARGET_SIZE as f32 / width as f32;
            (TARGET_SIZE, (height as f32 * ratio) as usize)
        } else {
            let ratio: f32 = TARGET_SIZE as f32 / height as f32;
            ((width as f32 * ratio) as usize, TARGET_SIZE)
        };
        let resized_img = img.resize(
            target_width as u32,
            target_height as u32,
            FilterType::Lanczos3,
        );
        Ok((
            resized_img.to_rgb8().to_vec(),
            resized_img.width() as usize,
            resized_img.height() as usize,
        ))
    } else {
        Ok((img.to_rgb8().to_vec(), width, height))
    }
}

fn compress(
    resized_img_data: Vec<u8>,
    target_width: usize,
    target_height: usize,
) -> Result<Vec<u8>, String> {
    let mut comp = Compress::new(ColorSpace::JCS_RGB);
    comp.set_scan_optimization_mode(ScanMode::AllComponentsTogether);
    comp.set_quality(70.0);

    comp.set_size(target_width, target_height);

    comp.set_mem_dest();
    comp.start_compress();

    let mut line = 0;
    loop {
        if line > target_height - 1 {
            break;
        }
        comp.write_scanlines(
            &resized_img_data[line * target_width * 3..(line + 1) * target_width * 3],
        );
        line += 1;
    }
    comp.finish_compress();

    let compressed = comp
        .data_to_vec()
        .map_err(|_| "data_to_vec failed".to_string())?;
    Ok(compressed)
}

fn pause() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();

    let _ = stdin.read(&mut [0u8]).unwrap();
}
