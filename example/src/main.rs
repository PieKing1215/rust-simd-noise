use std::fmt::{Display, Formatter, Result};

use clap::{Parser, Subcommand, ValueEnum};
use minifb::{Key, Window, WindowOptions};
use simdnoise::CellDistanceFunction;

const FPS: u64 = 60;

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;

const OFFSET_X: f32 = 1200.0;
const OFFSET_Y: f32 = 200.0;
const OFFSET_Z: f32 = 1.0;
const SCALE_MIN: f32 = 0.0;
const SCALE_MAX: f32 = 255.0;
const DEPTH: usize = 1;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser, default_value_t = WIDTH, help="The width of the generated image", global=true)]
    pub width: usize,

    #[clap(long, value_parser, default_value_t = HEIGHT, help="The height of the generated image", global=true)]
    pub height: usize,

    #[clap(long, value_parser, default_value_t = Dimension::Three, help="The number of dimensions of the generated noice", global=true)]
    pub dimension: Dimension,

    #[clap(
        long,
        value_parser,
        default_value_t = 8,
        help = "The initial seed value",
        global = true
    )]
    pub seed: i32,

    #[clap(
        long,
        value_parser,
        default_value_t = false,
        help = "Use an offset",
        global = true
    )]
    pub offset: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum Dimension {
    One,
    Two,
    Three,
}

impl Display for Dimension {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let num = match self {
            Dimension::One => "one",
            Dimension::Two => "two",
            Dimension::Three => "three",
        };
        write!(f, "{}", num)
    }
}

// should we expose ValueEnum to CellDistanceFunction?
#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum Distance {
    Euclidean,
    Manhattan,
    Natural,
}

impl From<Distance> for CellDistanceFunction {
    fn from(distance: Distance) -> Self {
        match distance {
            Distance::Euclidean => CellDistanceFunction::Euclidean,
            Distance::Manhattan => CellDistanceFunction::Manhattan,
            Distance::Natural => CellDistanceFunction::Natural,
        }
    }
}

impl Display for Distance {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let num = match self {
            Distance::Euclidean => "Euclidean",
            Distance::Manhattan => "Manhattan",
            Distance::Natural => "Natural",
        };
        write!(f, "{}", num)
    }
}
impl Distance {
    pub fn into(&self) -> CellDistanceFunction {
        match self {
            Distance::Euclidean => CellDistanceFunction::Euclidean,
            Distance::Manhattan => CellDistanceFunction::Manhattan,
            Distance::Natural => CellDistanceFunction::Natural,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Cellular {
        #[clap(long, value_parser, default_value_t = Distance::Euclidean, help="The distance function", global=true)]
        distance: Distance,
        #[arg(short, long, value_parser, default_value_t = 1.2)]
        frequency: f32,
        #[arg(short, long, value_parser, default_value_t = 1.2)]
        jitter: f32,
        //@TODO: index0/1
    },
    #[command(arg_required_else_help = true)]
    Ridge {
        #[arg(short, long, value_parser, default_value_t = 1.2)]
        frequency: f32,
        #[arg(short, long, value_parser, default_value_t = 8)]
        octaves: u8,
    },
}

fn main() {
    let args = Args::parse();
    let width = args.width;
    let height = args.height;
    let buffer: Vec<u32> = match (args.command, args.dimension, args.offset) {
        (
            Commands::Cellular {
                frequency: _,
                jitter: _,
                distance: _,
            },
            Dimension::One,
            _,
        ) => {
            unimplemented!()
        }
        (
            Commands::Cellular {
                frequency,
                jitter,
                distance,
            },
            Dimension::Two,
            false,
        ) => {
            let noise = simdnoise::NoiseBuilder::cellular_2d(width, height)
                .with_freq(frequency)
                .with_jitter(jitter)
                .with_distance_function(distance.into())
                .with_seed(args.seed)
                .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }
        (
            Commands::Cellular {
                frequency,
                jitter,
                distance,
            },
            Dimension::Two,
            true,
        ) => {
            let noise =
                simdnoise::NoiseBuilder::cellular_2d_offset(OFFSET_X, width, OFFSET_Y, height)
                    .with_freq(frequency)
                    .with_jitter(jitter)
                    .with_distance_function(distance.into())
                    .with_seed(args.seed)
                    .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }
        (
            Commands::Cellular {
                frequency,
                jitter,
                distance,
            },
            Dimension::Three,
            false,
        ) => {
            let noise = simdnoise::NoiseBuilder::cellular_3d(width, height, DEPTH)
                .with_freq(frequency)
                .with_jitter(jitter)
                .with_distance_function(distance.into())
                .with_seed(args.seed)
                .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }
        (
            Commands::Cellular {
                frequency,
                jitter,
                distance,
            },
            Dimension::Three,
            true,
        ) => {
            let noise = simdnoise::NoiseBuilder::cellular_3d_offset(
                OFFSET_X, width, OFFSET_Y, height, OFFSET_Z, DEPTH,
            )
            .with_freq(frequency)
            .with_jitter(jitter)
            .with_distance_function(distance.into())
            .with_seed(args.seed)
            .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }

        (Commands::Ridge { frequency, octaves }, Dimension::One, false) => {
            let noise = simdnoise::NoiseBuilder::ridge_1d(width)
                .with_freq(frequency)
                .with_seed(args.seed)
                .with_octaves(octaves)
                .generate_scaled(SCALE_MIN, SCALE_MAX);
            let x: Vec<u32> = noise.iter().map(|x| *x as u32).collect();
            let mut xy = Vec::with_capacity(x.len() * height);
            for _i in 0..height {
                xy.extend_from_slice(x.as_slice());
            }
            xy
        }
        (Commands::Ridge { frequency, octaves }, Dimension::One, true) => {
            let noise = simdnoise::NoiseBuilder::ridge_1d_offset(OFFSET_X, width)
                .with_freq(frequency)
                .with_seed(args.seed)
                .with_octaves(octaves)
                .generate_scaled(SCALE_MIN, SCALE_MAX);
            let x: Vec<u32> = noise.iter().map(|x| *x as u32).collect();
            let mut xy = Vec::with_capacity(x.len() * height);
            for _i in 0..height {
                xy.extend_from_slice(x.as_slice());
            }
            xy
        }
        (Commands::Ridge { frequency, octaves }, Dimension::Two, false) => {
            let noise = simdnoise::NoiseBuilder::ridge_2d(width, height)
                .with_freq(frequency)
                .with_seed(args.seed)
                .with_octaves(octaves)
                .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }
        (Commands::Ridge { frequency, octaves }, Dimension::Two, true) => {
            let noise = simdnoise::NoiseBuilder::ridge_2d_offset(OFFSET_X, width, OFFSET_Y, height)
                .with_freq(frequency)
                .with_seed(args.seed)
                .with_octaves(octaves)
                .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }
        (Commands::Ridge { frequency, octaves }, Dimension::Three, false) => {
            let noise = simdnoise::NoiseBuilder::ridge_3d(width, height, DEPTH)
                .with_freq(frequency)
                .with_seed(args.seed)
                .with_octaves(octaves)
                .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }
        (Commands::Ridge { frequency, octaves }, Dimension::Three, true) => {
            let noise = simdnoise::NoiseBuilder::ridge_3d_offset(
                OFFSET_X, width, OFFSET_Y, height, OFFSET_Z, DEPTH,
            )
            .with_freq(frequency)
            .with_seed(args.seed)
            .with_octaves(octaves)
            .generate_scaled(SCALE_MIN, SCALE_MAX);
            noise.iter().map(|x| *x as u32).collect()
        }
        _ => {
            unimplemented!();
        }
    };
    let mut window = Window::new(
        "Test - ESC to exit",
        width,
        height,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });
    let refresh_interval = 1_000_000 / FPS;
    window.limit_update_rate(Some(std::time::Duration::from_micros(refresh_interval)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buffer, width, height).unwrap();
    }
}
