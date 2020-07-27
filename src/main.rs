use clap::{App, Arg, Values};
use std::process::{Command, exit};

static AVAILABLE_ALLOCATORS: [&str; 3] = ["libc", "lrmalloc.rs", "jemalloc"];

mod benchmark;

fn main() {



    let matches = App::new("lrmalloc.rs benchmarking utility")
        .author("Joshua Radin <jradin2@u.rochester.edu>")
        .version("0.1.0")
        .about("Runs benchmarks for different allocators, namely lrmalloc.rs, jemalloc, and libc")
        .arg(
            Arg::with_name("allocator")
                .short('a')
                .long("allocator")
                .takes_value(true)
                .min_values(1)
                .multiple(true)
                .about("The allocator(s) to test. If no allocators are specified, all are tested")
        )
        .arg(
            Arg::with_name("benchmark")
                .short('b')
                .long("benchmark")
                .takes_value(true)
                .min_values(1)
                .multiple(true)
                .about("The benchmarks to test. If not benchmarks are specified, all are run")
        )
        .subcommand(
            App::new("clean")
                .about("Cleans the allocators, forcing a remake of the allocators")
        ).get_matches();

    // println!("Current directory: {:?}", std::env::current_dir());

    if matches.subcommand_matches("clean").is_some() {
        let cmd = Command::new("find")
            .current_dir("./allocators/target")
            .arg(".")
            .args(&["-name", "*.a"])
            .output()
            .unwrap();

        let files = String::from_utf8(cmd.stdout).unwrap()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        println!("Files to remove: {:?}", files);

        Command::new("rm").current_dir("./allocators/target").args(files).spawn().unwrap();

        Command::new("cargo").current_dir("./allocators/lrmalloc.rs").arg("clean").spawn().unwrap();
        Command::new("rm").current_dir("./allocators/jemalloc/lib").arg("libjemalloc.a").spawn().unwrap();
        return;
    }

    let allocators = matches.values_of("allocator");
    let allocators: Vec<&str> = match allocators {
        None => {
            AVAILABLE_ALLOCATORS.to_vec()
        },
        Some(listed) => {
            let listed = listed.collect::<Vec<&str>>();
            for allocator in &listed {
                if !AVAILABLE_ALLOCATORS.contains(allocator) {
                    eprintln!("Not a valid allocator: {}", *allocator);
                    exit(2);
                }
            }
            listed
        },
    };

    let benchmarks = matches.values_of("benchmark");

    for allocator in allocators {
        let allocator_library = get_allocator_lib_file(allocator);
    }

}

fn get_allocator_lib_file(allocator_name: &str) -> Option<&str> {
    match allocator_name {
        "libc" => {
            None
        },
        "lrmalloc.rs" => {
            Some("liblrmalloc_rs_global.a")
        },
        "jemalloc" => {
            Some("libjemalloc.a")
        }
        a => {
            panic!("{} is not a registered allocator!", a)
        }
    }
}
