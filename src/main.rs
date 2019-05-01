use std::io::prelude::*;

use std::io::BufReader;
use std::fs::File;
use std::io::SeekFrom;
use std::path::Path;

use bytepack::LEUnpacker;

extern crate image;

#[macro_use]
extern crate structopt;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, Copy, Clone)]
struct PhotonHeader {
    magic: [u8; 8],
    size_x: f32,
    size_y: f32,
    size_z: f32,
    padding0: [u32; 3],
    layer_height: f32,
    exposure_time: f32,
    exposure_time_bottom: f32,
    off_time: f32,
    bottom_layers: i32,
    screen_width: i32,
    screen_height: i32,
    preview_high_res_offset: i32,
    layer_defs_offset: i32,
    num_layers: i32,
    preview_low_res_offset: i32,
    unknown6: i32,
    projection_type: i32,
    padding1: [u32; 6],
}

#[derive(Debug)]
struct LayerDef {
    layer_height: f32,
    exposure_time: f32,
    off_time: f32,
    data_offset: i32,
    data_length: i32,
    padding: [i32; 4],
}

#[derive(Debug)]
struct PhotonFile {
    header: PhotonHeader,
    layer_defs: Vec<LayerDef>,
    layers: Vec<Vec<u8>>,
}

fn read_header(mut file: &File) -> std::io::Result<PhotonHeader> {
    let mut magic = [0u8; 8];
    file.read(&mut magic)?;

    let size_x = file.unpack()?;
    let size_y = file.unpack()?;
    let size_z = file.unpack()?;
    let padding0 = file.unpack()?;
    let layer_height = file.unpack()?;
    let exposure_time = file.unpack()?;
    let exposure_time_bottom = file.unpack()?;
    let off_time = file.unpack()?;
    let bottom_layers = file.unpack()?;
    let screen_width = file.unpack()?;
    let screen_height = file.unpack()?;
    let preview_high_res_offset = file.unpack()?;
    let layer_defs_offset = file.unpack()?;
    let num_layers = file.unpack()?;
    let preview_low_res_offset = file.unpack()?;
    let unknown6 = file.unpack()?;
    let projection_type = file.unpack()?;
    let padding1 = file.unpack()?;

    let header = PhotonHeader {
        magic,
        size_x,
        size_y,
        size_z,
        padding0,
        layer_height,
        exposure_time,
        exposure_time_bottom,
        off_time,
        bottom_layers,
        screen_width,
        screen_height,
        preview_high_res_offset,
        layer_defs_offset,
        num_layers,
        preview_low_res_offset,
        unknown6,
        projection_type,
        padding1,
    };
    Ok(header)
}

fn read_layer_defs(mut file: &File, header: PhotonHeader) -> std::io::Result<Vec<LayerDef>> {
    file.seek(SeekFrom::Start(header.layer_defs_offset as u64))?;

    let mut layer_defs = Vec::new();

    for _ in 0..header.num_layers {
        let layer_height = file.unpack()?;
        let exposure_time = file.unpack()?;
        let off_time = file.unpack()?;
        let data_offset = file.unpack()?;
        let data_length = file.unpack()?;
        let padding = file.unpack()?;

        let layer_def = LayerDef {
            layer_height,
            exposure_time,
            off_time,
            data_offset,
            data_length,
            padding,
        };
        layer_defs.push(layer_def);
    }

    Ok(layer_defs)
}

fn read_layers(mut file: &File, header: &PhotonHeader, layer_defs: &Vec<LayerDef>) -> std::io::Result<Vec<Vec<u8>>> {
    let mut layers = Vec::with_capacity(header.num_layers as usize);
    let mut layer_no = 0;
    for layer_def in layer_defs {
        file.seek(SeekFrom::Start(layer_def.data_offset as u64))?;
        let mut reader = BufReader::new(file);
        let mut buf = vec![0u8; layer_def.data_length as usize];
        reader.read_exact(&mut buf)?;
        let mut layer = Vec::with_capacity((header.screen_height * header.screen_width) as usize);
        for b in buf {
            let color = b >> 7;
            let length = b & 127;
            for _ in 0..length {
                layer.push(color * 255);
            }
        }
        layers.push(layer);
        layer_no += 1;
        println!("layer: {}/{}", layer_no, layer_defs.len())
    }
    Ok(layers)
}

fn read_file(file: &File) -> std::io::Result<PhotonFile> {
    let header = read_header(file)?;
    let layer_defs = read_layer_defs(file, header)?;
    let layers = read_layers(file, &header, &layer_defs)?;
    let photon_file = PhotonFile { 
        header, 
        layer_defs, 
        layers,
    };
    Ok(photon_file)
}

#[derive(StructOpt)]
struct CliArgs {
    /// The path to the file to read
    #[structopt(parse(from_os_str))]
    input: std::path::PathBuf,

    exportLayer: i8,
}

fn main() -> std::io::Result<()> {
    let args = CliArgs::from_args();
    let file = File::open(&args.input)?;

    let photon_file = read_file(&file)?;
    if photon_file.layers.len() > 0 {
        image::save_buffer(&Path::new("layer.png"), 
            &photon_file.layers[0], 
            photon_file.header.screen_width as u32, 
            photon_file.header.screen_height as u32, 
            image::ColorType::Gray(8))?;
    }

    Ok(())
}
