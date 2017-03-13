extern crate srt;
extern crate ebustl;

use std::env;
use std::process;
use std::error::Error;
use srt::{Srt, parse_srt_from_file};
use ebustl::{Stl, Time, TtiFormat};

fn print_usage() {
    println!("sub-converter input.srt output.stl\n");
}

fn convert_srt_to_stl(srt: Srt) -> Result<Stl, String> {
    let mut stl = Stl::new();
    let fps = 25; // Assume 25 fps video
    for sub in srt.subs {
        stl.add_sub(Time {
                        hours: sub.start_time.hours,
                        minutes: sub.start_time.minutes,
                        seconds: sub.start_time.seconds,
                        frames: (sub.start_time.milliseconds * fps / 1000) as u8,
                    },
                    Time {
                        hours: sub.end_time.hours,
                        minutes: sub.end_time.minutes,
                        seconds: sub.end_time.seconds,
                        frames: (sub.end_time.milliseconds * fps / 1000) as u8,
                    },
                    &sub.text,
                    TtiFormat {
                        jc: 2,
                        vp: 19,
                        dh: true,
                    });
    }
    Ok(stl)
}

fn main() {
    if env::args().count() != 3 {
        print_usage();
        process::exit(1);
    }
    let input_filename = env::args().nth(1).unwrap();
    let output_filename = env::args().nth(2).unwrap();
    let res = parse_srt_from_file(&input_filename)
        .map_err(|err| err.description().to_string())
        .and_then(convert_srt_to_stl)
        .and_then(|stl| {
                      stl.write_to_file(&output_filename).map_err(|err| {
                                                                      err.description().to_string()
                                                                  })
                  });
    if res.is_err() {
        print_usage();
        println!("Error: {}\n", res.err().unwrap());
        process::exit(1);
    }
}
