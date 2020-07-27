use clap::{App, Arg, Values};
use std::process::{Command, exit};
use std::ffi::OsString;
use crate::benchmark::Benchmark;
use std::path::PathBuf;

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
        .arg(
            Arg::with_name("verbose")
                .short('v')
                .long("verbose")
                .about("Shows verbose output")
                .takes_value(false)
        )
        .subcommand(
            App::new("clean")
                .about("Cleans the allocators, forcing a remake of the allocators")
        ).get_matches();

    // println!("Current directory: {:?}", std::env::current_dir());

    let verbose = matches.is_present("verbose");

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

    let available_benchmarks = benchmark::get_available_benchmarks().unwrap();
    if verbose {
        println!("All available benchmarks = {:?}", available_benchmarks);
    }

    let benchmarks = matches.values_of("benchmark");
    let running_benchmarks: Vec<_> =
        if let Some(benchmarks) = benchmarks {
            let mut out = vec![];
            for benchmark in benchmarks {
                let os_string = OsString::from(benchmark);
                if !available_benchmarks.contains(&os_string) {
                    eprintln!("{} is not a valid benchmark. Valid benchmarks = {:?}", benchmark, available_benchmarks);
                    exit(2);
                }
                let bench = Benchmark::new(PathBuf::from(os_string));
                out.push(bench);
            }
            out
        } else {
            available_benchmarks.into_iter().map(|p| Benchmark::new(PathBuf::from(p))).collect()
        };


    let allocator_libs: Vec<Option<String>> =
        allocators.into_iter().map(|s|
            get_allocator_lib_file(s))
            .map(|o|
                o.map(|s| s.to_string())
            )
            .collect();

    for benchmark in running_benchmarks {
        benchmark.create_object_file();
        benchmark.create_binaries_for(&allocator_libs);
    }

}

fn get_allocator_lib_file(allocator_name: &str) -> Option<&str> {
    match allocator_name {
        "libc" => {
            None
        },
        "lrmalloc.rs" => {
            Some("lrmalloc_rs_global")
        },
        "jemalloc" => {
            Some("jemalloc")
        }
        a => {
            panic!("{} is not a registered allocator!", a)
        }
    }
}
