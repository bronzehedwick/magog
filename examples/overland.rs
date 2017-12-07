///! Generate a base bitmap for drawing an overland map with a paint program.
extern crate calx;
extern crate euclid;
extern crate image;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate world;

use calx::{hex_disc, CellVector, Prefab, ProjectedImage, SRgba};
use euclid::vec2;
use image::{GenericImage, Pixel};
use std::collections::HashMap;
use std::iter::FromIterator;
use structopt::StructOpt;
use world::{Location, Sector, Terrain};

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(subcommand)] cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "generate", help = "Generate a blank overland map image")]
    Generate {
        #[structopt(short = "w", long = "width", default_value = "8", help = "Width in sectors")]
        width: u32,

        #[structopt(short = "h", long = "height", default_value = "9", help = "Height in sectors")]
        height: u32,

        #[structopt(long = "minimap", default_value = "false", help = "Use minimap projection")]
        minimap: bool,

        #[structopt(help = "Output PNG file", default_value = "overland_base.png")]
        output: String,
    },

    #[structopt(name = "convert",
                help = "Convert map from one projection to another and normalize the checkerboard pattern")]
    Convert {
        #[structopt(help = "Input file")]
        input: String,

        #[structopt(long = "input_minimap", default_value = "false", help = "Input file has minimap projection")]
        input_minimap: bool,

        #[structopt(help = "Output file (if different from input)")]
        output: Option<String>,

        #[structopt(long = "output_minimap", default_value = "false", help = "Use minimap projection in output file")]
        output_minimap: bool,
    },
}

fn default_map(width: u32, height: u32) -> Prefab<Terrain> {
    fn p(loc: Location) -> CellVector { vec2(loc.x as i32, loc.y as i32) }

    let mut terrain = HashMap::new();

    for &loc in &overland_locs(width, height) {
        for edge in hex_disc(loc, 1) {
            terrain.insert(p(edge), Terrain::Water);
        }
        terrain.insert(p(loc), Terrain::Grass);
    }

    Prefab::from_iter(terrain.into_iter())
}

fn terrain_to_color((pos, terrain): (CellVector, Terrain)) -> (CellVector, SRgba) {
    let sec = Location::new(pos.x as i16, pos.y as i16, 0).sector();
    let is_dark = (sec.x + sec.y) % 2 != 0;

    (
        pos,
        if is_dark {
            terrain.dark_color()
        } else {
            terrain.color()
        },
    )
}

fn overland_locs(width: u32, height: u32) -> Vec<Location> {
    let mut ret = Vec::new();
    for y in 0..(height as i16) {
        for x in 0..(width as i16) {
            let sec = Sector::new(x, y, 0);
            for loc in sec.iter() {
                ret.push(loc);
            }
        }
    }
    ret
}

/*
fn generate(width: u32, height: u32, output: &str) {
    let area = location_area(width, height);

    println!("Origin {{ x: {}, y: {} }}", area.origin.x, area.origin.y);

    let mut buf = image::ImageBuffer::new(area.size.width as u32, (area.size.height + 1) as u32);

    for (x, y, p) in buf.enumerate_pixels_mut() {
        let loc = Location::new(x as i16 + area.origin.x, y as i16 + area.origin.y, 0);
        let sec = loc.sector();
        let terrain = if valid_sector(sec, width, height) {
            Terrain::Grass
        } else if hex_disc(loc, 1).any(|loc| valid_sector(loc.sector(), width, height)) {
            // Not a valid sector, but touching one.
            Terrain::Water
        } else {
            *p = image::Rgb::from_channels(0x00, 0x00, 0x00, 0xff);
            continue;
        };

        let color = if sec.is_dark() {
            terrain.dark_color()
        } else {
            terrain.color()
        };

        *p = image::Rgb::from_channels(color.r, color.g, color.b, 0xff);
    }

    // XXX: Hacky way to force terrain colors into image palette
    // Didn't find direct indexed palette manipulation in piston-image.
    for (x, t) in Terrain::iter().filter(|t| t.is_regular()).enumerate() {
        let light = t.color();
        let dark = t.dark_color();
        let y = buf.height() - 1;
        buf.put_pixel(
            x as u32 * 2,
            y,
            image::Rgb::from_channels(light.r, light.g, light.b, 0xff),
        );
        buf.put_pixel(
            x as u32 * 2 + 1,
            y,
            image::Rgb::from_channels(dark.r, dark.g, dark.b, 0xff),
        );
    }

    image::save_buffer(
        output,
        &buf,
        buf.width(),
        buf.height(),
        image::ColorType::RGB(8),
    ).unwrap();
}

fn normalize(width: u32, height: u32, input: &str, out_path: &str) {
    let input = image::open(input).expect(&format!("Unable to load '{}'", input));
    let mut output: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        image::ImageBuffer::new(input.width(), input.height());

    let area = location_area(width, height);

    for y in 0..input.height() {
        for x in 0..input.width() {
            let in_map_space = y < input.height() - 1;

            let p = input.get_pixel(x, y);
            let sector =
                Location::new((x as i16) + area.origin.x, (y as i16) + area.origin.y, 0).sector();

            let output_pixel = if in_map_space {
                if let Some(t) = Terrain::from_color(p.into()) {
                    if sector.is_dark() {
                        t.dark_color()
                    } else {
                        t.color()
                    }.into()
                } else {
                    p
                }
            } else {
                p
            };

            output.put_pixel(x, y, output_pixel);
        }
    }

    image::save_buffer(
        out_path,
        &output,
        output.width(),
        output.height(),
        image::ColorType::RGBA(8),
    ).unwrap();
}

fn minimap(width: u32, height: u32, input: &str, out_path: &str) {
    let area = location_area(width, height);

    let input = image::open(input).expect(&format!("Unable to load '{}'", input));

    let mut output: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image::ImageBuffer::new(
        width * (world::SECTOR_WIDTH as u32) * 2,
        height * (world::SECTOR_HEIGHT as u32) * 2,
    );

    for (x, y, p) in output.enumerate_pixels_mut() {
        let y = y as i32;
        let x = x as i32;

        let column = x / 2;
        let row = (y - column) / 2;
        let loc_x = (column + row) as i32 - (area.origin.x as i32);
        let loc_y = row as i32 - (area.origin.y as i32);

        if loc_x >= 0 && loc_y >= 0 && loc_x < input.width() as i32 && loc_y < input.height() as i32
        {
            let col = input.get_pixel(loc_x as u32, loc_y as u32);
            // Remove the chessboard effect from terrain colors.
            let col = if let Some(t) = Terrain::from_color(col.into()) {
                t.color().into()
            } else {
                col
            };
            *p = col;
        }
    }

    image::save_buffer(
        out_path,
        &output,
        output.width(),
        output.height(),
        image::ColorType::RGBA(8),
    ).unwrap();
}
*/

fn generate(width: u32, height: u32, is_minimap: bool, output_path: String) {
    let map: Prefab<SRgba> = default_map(width, height)
        .into_iter()
        .map(terrain_to_color)
        .collect();
}

fn convert(
    input_path: String,
    input_is_minimap: bool,
    output_path: Option<String>,
    output_is_minimap: bool,
) {
    unimplemented!();
}

fn main() {
    let opt = Opt::from_args();
    match opt.cmd {
        Command::Generate {
            width,
            height,
            minimap,
            output,
        } => generate(width, height, minimap, output),
        Command::Convert {
            input,
            input_minimap,
            output,
            output_minimap,
        } => convert(input, input_minimap, output, output_minimap),
    }
}
