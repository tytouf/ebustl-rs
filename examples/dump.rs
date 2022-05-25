extern crate ebustl;

use ebustl::parse_stl_from_file;
use std::env;
use std::process;

fn print_usage() {
    println!("dump file.stl\n");
}

fn main() {
    if env::args().count() != 2 {
        print_usage();
        process::exit(1);
    }
    let input_filename = env::args().nth(1).unwrap();
    let stl = parse_stl_from_file(&input_filename).map_err(|err| err.to_string());
    println!("{:?}", stl);
}
