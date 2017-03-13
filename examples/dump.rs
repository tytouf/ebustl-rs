extern crate ebustl;

use std::env;
use std::process;
use std::error::Error;
use ebustl::parse_stl_from_file;

fn print_usage() {
    println!("dump file.stl\n");
}

fn main() {
    if env::args().count() != 2 {
        print_usage();
        process::exit(1);
    }
    let input_filename = env::args().nth(1).unwrap();
    let stl = parse_stl_from_file(&input_filename).map_err(|err| err.description().to_string());
    println!("{:?}", stl);
}
