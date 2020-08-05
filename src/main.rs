#![deny(unused_imports)]

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Write;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant};

use clap::{App, Arg};

use crate::age_checker::should_build;
use crate::benchmark::{Benchmark, BENCHMARK_DIR, LIBRARY_DIR};
use crate::grapher::Graph;

static AVAILABLE_ALLOCATORS: [&str; 4] =
    [
        "libc",
        "apfmalloc",
        "jemalloc",
        "lrmalloc"
    ];
const BINARY_DIR: &str = "./benchmarks/bin";
const BENCHMARK_RESULTS: &str = "./benchmarks/results";
mod benchmark;
mod age_checker;
mod grapher;

static DEBUG_MODE: AtomicBool = AtomicBool::new(false);
static DYNAMIC_MODE: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
const DYNAMIC_LIBRARY_EXTENSION: &str = ".dylib";
#[cfg(target_os = "linux")]
const DYNAMIC_LIBRARY_EXTENSION: &str = ".so";
#[cfg(target_os = "windows")]
const DYNAMIC_LIBRARY_EXTENSION: &str = ".dylib";

macro_rules! dict {
    ($($key:expr => $value:expr),*) => {
        {
            let mut ret = std::collections::HashMap::new();
            $(ret.insert($key, $value);)*
            ret
        }
    };
    ($($key:expr => $value:expr),*,) => {
        dict![$($key => $value),*]
    };
}

fn main() {

    let benchmark_param_list = dict![
        "t-test1"=> "10 {} 10000 10000 400",
        "t-test2"=> "10 {} 10000 10000 400",
    ];



    let matches = App::new("apfmalloc benchmarking utility")
        .author("Joshua Radin <jradin2@u.rochester.edu>")
        .version("0.1.0")
        .about("Runs benchmarks for different allocators, namely apfmalloc, jemalloc, and libc")
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
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .short('d')
                .about("Generate debug symbols in output")
                .takes_value(false)
        )
        .arg(
            Arg::with_name("threads")
                .long("threads")
                .short('t')
                .about("The maximum number of threads to test")
                .takes_value(true)
                .number_of_values(1)
                .default_value("16")
        )
        .arg(
            Arg::with_name("features")
                .long("features")
                .about("Set features for the apfmalloc build (track_allocation, no_met_stack)")
                .multiple_values(true)
                .min_values(1)
        )
        .arg(
            Arg::with_name("dynamic")
                .long("dynamic")
                .about("Use dynamic libraries instead of static")
        )
        .subcommand(
            App::new("clean")
                .about("Cleans the allocators, forcing a remake of the allocators")
        )
        .get_matches();

    // println!("Current directory: {:?}", std::env::current_dir());

    #[allow(unused)]
        let verbose = matches.is_present("verbose");

    if matches.is_present("debug") {
        DEBUG_MODE.store(true, Ordering::Release);
    }

    if matches.is_present("dynamic") {
        DYNAMIC_MODE.store(true, Ordering::Release);
    }

    macro_rules! vprint {
        ($($tokens:tt),+) => {
            if verbose {
                print!($($tokens),+)
            }
        };
    }
    macro_rules! vprintln {
        () => {
            vprint!("\n")
        };
        ($($tokens:tt),*) => {
            vprint!($($tokens),*);
            vprint!("\n")
        }
    }

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
        vprintln!("Files to remove: {:?}", files);

        // asdasd

        Command::new("rm").current_dir("./allocators/target").args(files).spawn().unwrap();

        Command::new("cargo").current_dir("./allocators/apfmalloc").arg("clean").spawn().unwrap();
        Command::new("rm").current_dir("./allocators/jemalloc/lib").arg("libjemalloc.a").spawn().unwrap();
        let _ = Command::new("make")
            .current_dir("./allocators/jemalloc")
            .arg("distclean")
            .spawn();

        Benchmark::clean_benchmarks();
        return;
    }

    let out_dir = Path::new("./allocators/target");
    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir).unwrap();
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

    if allocators.contains(&"jemalloc") {
        if !Path::new("./allocators/jemalloc/.git").exists() || !Path::new("./allocators/apfmalloc/.git").exists() || !Path::new("./allocators/lrmalloc/.git").exists() {
            vprintln!("Initializing the submodules...");
            Command::new("git")
                .arg("submodule")
                .arg("init")
                .status()
                .expect("Failed to initialize allocator directories");
        }

        if !Path::new("./allocators/jemalloc/Makefile").exists() ||
            !Path::new("./allocators/apfmalloc/Cargo.toml").exists() ||
            !Path::new("./allocators/lrmalloc/Makefile").exists() {
            vprintln!("Updating submodules...");
            Command::new("git")
                .arg("submodule")
                .arg("update")
                .arg("--remote")
                .status()
                .expect("Failed to initialize allocator repos");
        }


        if !Path::new("./allocators/jemalloc/Makefile").exists() {
            vprintln!("Configuring jemalloc...");

            if !Path::new("./allocators/jemalloc/configure").exists() {
                if !Path::new("./allocators/jemalloc/autogen.sh").exists() {
                    eprintln!("Neither the Makefile, configure, or autogen.sh files exist. Can not build jemalloc");
                    exit(5);
                }


                //let process = create_process_name_for_local("./allocators/jemalloc/autogen.sh").expect("Could not create a absolute path");
                // vprintln!("Attempting to run {:?}", process);
                Command::new("sh")
                    .arg("./autogen.sh")
                    .current_dir("./allocators/jemalloc")
                    .status()
                    .expect("Failed to execute process");
            } else {
                Command::new("./configure")
                    .current_dir("./allocators/jemalloc")
                    .arg("--without-export")
                    .arg("--disable-zone-allocator")
                    .status()
                    .expect("Failed to run the configure command");
            }
        }


        if should_build("jemalloc") || DYNAMIC_MODE.load(Ordering::Acquire) {
            vprintln!("Building jemalloc");

            while !Path::new("./allocators/jemalloc/Makefile").exists() {}
            let file_name = format!("libjemalloc{}", if DYNAMIC_MODE.load(Ordering::Acquire) {
                DYNAMIC_LIBRARY_EXTENSION
            } else {
                ".a"
            });
            if !DYNAMIC_MODE.load(Ordering::Acquire) {
                Command::new("make")
                    .current_dir("./allocators/jemalloc")
                    .arg("build_lib_static")
                    .status()
                    .unwrap();
            } else {
                Command::new("make")
                    .current_dir("./allocators/jemalloc")
                    .arg("build_lib_shared")
                    .status()
                    .unwrap();
            }
            let mut dest_path = PathBuf::from(out_dir.to_str().unwrap());
            dest_path.push(file_name.clone());
            Command::new("cp")
                .arg(format!("./allocators/jemalloc/lib/{}", file_name))
                .arg(dest_path)
                .status()
                .unwrap();
        }
    }

    if allocators.contains(&"apfmalloc") {

        println!("TARGET_APF = {:?}", option_env!("TARGET_APF"));
        let features = matches
            .values_of("features")
            .map_or(vec![],
                    |iter| {
                        let mut collected: Vec<&str> = iter.collect();
                        collected.insert(0, "--features");
                        collected
                    }
            );


        vprintln!("Creating apfmalloc");
        let file_name = format!("libapfmalloc{}", if DYNAMIC_MODE.load(Ordering::Acquire) {
            DYNAMIC_LIBRARY_EXTENSION
        } else {
            ".a"
        });
        if is_debug() {
            vprintln!("Making debug version");
            Command::new("cargo")
                .arg("build")
                .arg("--workspace")
                .arg("--manifest-path")
                .arg("allocators/apfmalloc/Cargo.toml")
                .args(features)
                .status()
                .unwrap();
            let mut dest_path = PathBuf::from(out_dir.to_str().unwrap());
            dest_path.push(file_name.clone());
            Command::new("cp")
                .arg(format!("allocators/apfmalloc/target/debug/{}", file_name))
                .arg(dest_path.to_str().unwrap())
                .status()
                .unwrap();
        } else {
            Command::new("cargo")
                .arg("build")
                .arg("--workspace")
                .arg("--release")
                .arg("--manifest-path")
                .arg("allocators/apfmalloc/Cargo.toml")
                .args(features)
                .status()
                .unwrap();
            let mut dest_path = PathBuf::from(out_dir.to_str().unwrap());
            dest_path.push(file_name.clone());
            Command::new("cp")
                .arg(format!("allocators/apfmalloc/target/release/{}", file_name))
                .arg(dest_path.to_str().unwrap())
                .status()
                .unwrap();

        }
    }

    if allocators.contains(&"lrmalloc") {
        if should_build("lrmalloc") || DYNAMIC_MODE.load(Ordering::Acquire) {
            vprintln!("Building lrmalloc");

            let file_name = format!("lrmalloc{}", if DYNAMIC_MODE.load(Ordering::Acquire) {
                DYNAMIC_LIBRARY_EXTENSION
            } else {
                ".a"
            });
            if !DYNAMIC_MODE.load(Ordering::Acquire) {
                if !Command::new("make")
                    .current_dir("./allocators/lrmalloc")
                    .arg("lrmalloc.a")
                    .status()
                    .unwrap().success() {
                    return;
                }
            } else if !Command::new("make")
                .current_dir("./allocators/lrmalloc")
                .arg("lrmalloc.so")
                .status()
                .unwrap().success() {
                return;
            }
            let mut dest_path = PathBuf::from(out_dir.to_str().unwrap());
            dest_path.push(format!("lib{}", file_name));
            Command::new("cp")
                .arg(format!("./allocators/lrmalloc/{}", file_name))
                .arg(dest_path)
                .status()
                .unwrap();
        }
    }


    let available_benchmarks = benchmark::get_available_benchmarks().unwrap();
    vprintln!("All available benchmarks = {:?}", available_benchmarks);
    let benchmark_names =
        available_benchmarks.iter()
            .map(|s| PathBuf::from(s.clone()))
            .map(|p| p.file_name().unwrap().to_os_string())
            .map(|s| s.into_string().unwrap())
            .collect::<Vec<_>>();

    let benchmarks = matches.values_of("benchmark");
    let mut running_benchmarks: Vec<_> =
        if let Some(benchmarks) = benchmarks {
            let mut out = vec![];
            for benchmark in benchmarks {
                let benchmark = benchmark.trim_end_matches("\"");
                if benchmark == "none" {
                    return;
                }

                let contains = benchmark_names.contains(&benchmark.to_string());
                if !contains {
                    eprintln!("{} is not a valid benchmark. Valid benchmarks = {:?}", benchmark, available_benchmarks);
                    exit(2);
                }
                let bench = Benchmark::new(PathBuf::from_iter(&[BENCHMARK_DIR, benchmark]));
                out.push(bench);
            }
            out
        } else {
            available_benchmarks.into_iter().map(|p| Benchmark::new(PathBuf::from(p))).collect()
        };
    running_benchmarks.sort_by(|b1, b2| b1.get_name().cmp(&b2.get_name()));


    let allocator_libs: Vec<Option<String>> =
        allocators.iter().map(|s|
            get_allocator_lib_file(*s))
            .map(|o|
                o.map(|s| s.to_string())
            )
            .collect();

    let max_threads: usize = matches.value_of("threads").unwrap().parse().expect("Invalid value for --threads entry");

    std::fs::create_dir_all(Path::new(BENCHMARK_RESULTS)).expect("Could not create benchmark result folder");

    for benchmark in running_benchmarks {
        benchmark.create_object_file().unwrap();
        let name = benchmark.get_name();
        match benchmark.create_binaries_for(&allocator_libs) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("{:?}", e);
                exit(3);
            },
        }
        if max_threads == 0 {
            continue;
        }

        let mut results = HashMap::new();
        for allocator in &allocators {
            results.insert(*allocator, vec![]);

            let binary_name = format!("{}-{}", name, get_allocator_lib_file(*allocator).unwrap_or("libc"));

            let mut binary_path = PathBuf::from(BINARY_DIR);
            binary_path.push(binary_name.clone());

            if !binary_path.exists() {
                panic!("Binary {} does not exist!", binary_name);
            }



            let output_file_name = format!("{}.txt", binary_name);
            let output_file_path = PathBuf::from_iter(&[BENCHMARK_RESULTS, output_file_name.as_str()]);
            let output_file =
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(output_file_path)
                    .expect(format!("Failed to create result file for {}", binary_name).as_str());

            let mut writer = BufWriter::new(output_file);


            for thread_count in 1..=max_threads {
                let params = benchmark_param_list[&*name];
                let args =
                    params.replace("{}", & *thread_count.to_string())
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>();

                println!("Running {} with {} threads", binary_path.as_path().file_name().unwrap().to_str().unwrap(), thread_count);
                writeln!(&mut writer, "-------------- [START] {} with {} threads --------------",
                         binary_name,
                         thread_count
                ).unwrap();

                let mut sum_throughput = 0.0;
                const NUM_TRIALS: usize = 3;
                for i in 0..NUM_TRIALS {
                    writeln!(
                        &mut writer,
                        "---- ))Start Iteration {} ----",
                        i
                    ).unwrap();


                    let mut command = Command::new(binary_path.to_str().unwrap());
                    command
                        .args(args.clone());
                    if DYNAMIC_MODE.load(Ordering::Acquire) {
                        if let Some(allocator) = get_allocator_lib_file(*allocator) {
                            let path = {
                                let mut path = PathBuf::from(LIBRARY_DIR);
                                path.push(format!("lib{}{}", allocator, DYNAMIC_LIBRARY_EXTENSION));
                                path
                            };

                            let path = path.canonicalize().unwrap_or_else(|_| panic!("Could not get canonical path for the dynamic library at {:?}", path));


                            #[cfg(target_os = "linux")]
                                command.env("LD_PRELOAD", path);
                            #[cfg(target_os = "macos")]
                                command.env("DYLD_INSERT_LIBRARIES", path);
                        }
                    }
                    let start = Instant::now();
                    let output = command.output().unwrap();



                    let duration = start.elapsed();
                    if !output.status.success() {
                        eprintln!("Program exited with code {}", output.status);
                        writeln!(
                            &mut writer,
                            "PROGRAM CRASHED\n-------------- [END] --------------"
                        ).unwrap();
                        return;
                    }
                    let output = from_utf8(&*output.stdout).expect("Output not in utf-8");
                    println!("{}", output);
                    writeln!(
                        &mut writer,
                        "{}",
                        output
                    ).unwrap();

                    let throughput = 1.0 / duration.as_secs_f64();
                    writeln!(
                        &mut writer,
                        "Throughput: {}",
                        throughput
                    ).unwrap();
                    sum_throughput += throughput;
                }
                let average = sum_throughput/(NUM_TRIALS as f64);
                writeln!(
                    &mut writer,
                    "#### Average Throughput: {} ####",
                    average
                ).unwrap();
                results.get_mut(allocator).unwrap().push(average);
                writeln!(
                    &mut writer,
                    "-------------- [END] --------------"
                ).unwrap();
            }

            writer.flush().unwrap();
        }
        let graph = Graph::new(name, results, max_threads);
        match graph.make_graph() {
            Ok(_) => {},
            Err(e) => {
                panic!("{:?}", e);
            },
        }

    }



}



fn is_debug() -> bool {
    DEBUG_MODE.load(Ordering::Acquire)
}

fn get_allocator_lib_file(allocator_name: &str) -> Option<&str> {
    match allocator_name {
        "libc" => {
            None
        },
        "apfmalloc" => {
            Some("apfmalloc")
        },
        "jemalloc" => {
            Some("jemalloc")
        },
        "lrmalloc" => {
            Some("lrmalloc")
        }
        a => {
            panic!("{} is not a registered allocator!", a)
        }
    }
}
