use clap::{App, Arg, Values};
use std::process::{Command, exit, Child};
use std::ffi::OsString;
use crate::benchmark::Benchmark;
use std::path::{PathBuf, Path};
use std::io::{Error, BufWriter};
use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::age_checker::should_build;
use std::fs::{File, OpenOptions};
use std::iter::FromIterator;
use std::io::Write;

static AVAILABLE_ALLOCATORS: [&str; 3] =
    [
        "libc",
        "lrmalloc.rs",
        "jemalloc"
    ];
const BINARY_DIR: &str = "./benchmarks/bin";
const BENCHMARK_RESULTS: &str = "./benchmarks/results";
mod benchmark;
mod age_checker;

static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

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

fn create_process_name_for_local(local_path: &str) -> std::io::Result<OsString> {
    let path = Path::new(local_path);
    if !path.is_absolute() {
        std::fs::canonicalize(local_path).map(|path| path.into_os_string())
    } else {
        Ok(path.as_os_str().to_os_string())
    }
}

fn main() {

    let benchmark_param_list = dict![
        "t-test1"=> "10 {} 10000 10000 400",
        "t-test2"=> "10 {} 10000 10000 400",
    ];



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
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .short('d')
                .about("Generate debug symbols in output")
                .takes_value(false)
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .short('t')
                .about("The maximum number of threads to test")
                .takes_value(true)
                .number_of_values(1)
                .default_value("16")
        )
        .subcommand(
            App::new("clean")
                .about("Cleans the allocators, forcing a remake of the allocators")
        )
        .get_matches();

    // println!("Current directory: {:?}", std::env::current_dir());

    let verbose = matches.is_present("verbose");

    if matches.is_present("debug") {
        DEBUG_MODE.store(true, Ordering::Release);
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

        Command::new("cargo").current_dir("./allocators/lrmalloc.rs").arg("clean").spawn().unwrap();
        Command::new("rm").current_dir("./allocators/jemalloc/lib").arg("libjemalloc.a").spawn().unwrap();
        let _ = Command::new("make")
            .current_dir("./allocators/jemalloc")
            .arg("distclean")
            .spawn();
        return;
    }

    let out_dir = Path::new("./allocators/target");
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
        }

        Command::new("./configure")
            .current_dir("./allocators/jemalloc")
            .arg("--without-export")
            .arg("--disable-zone-allocator")
            .status()
            .expect("Failed to run the configure command");
    }


    if should_build("jemalloc") {
        vprintln!("Building jemalloc");
        while !Path::new("./allocators/jemalloc/Makefile").exists() {

        }
        Command::new("make")
            .current_dir("./allocators/jemalloc")
            .arg("build_lib_static")
            .status()
            .unwrap();
        let mut dest_path = PathBuf::from(out_dir.to_str().unwrap());
        dest_path.push("libjemalloc.a");
        Command::new("cp")
            .arg("./allocators/jemalloc/lib/libjemalloc.a")
            .arg(dest_path)
            .status()
            .unwrap();
    }

    if should_build("lrmalloc.rs") {
        vprintln!("Creating lrmalloc.rs");
        if is_debug() {
            Command::new("cargo")
                .arg("build")
                .arg("--manifest-path")
                .arg("allocators/lrmalloc.rs/lrmalloc-rs-global/Cargo.toml")
                .status()
                .unwrap();
            let mut dest_path = PathBuf::from(out_dir.to_str().unwrap());
            dest_path.push("liblrmalloc_rs_global.a");
            Command::new("cp")
                .arg("allocators/lrmalloc.rs/target/debug/liblrmalloc_rs_global.a")
                .arg(dest_path.to_str().unwrap())
                .status()
                .unwrap();
        } else {
            Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--manifest-path")
                .arg("allocators/lrmalloc.rs/lrmalloc-rs-global/Cargo.toml")
                .status()
                .unwrap();
            let mut dest_path = PathBuf::from(out_dir.to_str().unwrap());
            dest_path.push("liblrmalloc_rs_global.a");
            Command::new("cp")
                .arg("allocators/lrmalloc.rs/target/release/liblrmalloc_rs_global.a")
                .arg(dest_path.to_str().unwrap())
                .status()
                .unwrap();
        }
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
    vprintln!("All available benchmarks = {:?}", available_benchmarks);


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


        for allocator in &allocators {
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
                let params = benchmark_param_list[name.as_str()];
                let args =
                    params.replace("{}", & *thread_count.to_string())
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>();

                println!("Running {}", binary_path.as_path().file_name().unwrap().to_str().unwrap());
                writeln!(&mut writer, "-------------- [START] {} with {} threads --------------",
                    binary_name,
                    thread_count
                ).unwrap();

                let mut sum_throughput = 0.0;
                const NUM_TRIALS: usize = 3;
                for i in 0..NUM_TRIALS {



                    let status =
                        Command::new(binary_path.to_str().unwrap())
                            .args(args.clone())
                            .status()
                            .unwrap();

                    println!("Program exited with status {}", status);
                    if !status.success() {
                        eprintln!("Program failed!");
                        exit(status.code().unwrap_or(-1))
                    }
                }
            }
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
