use std::path::{Path, PathBuf};
use std::ffi::OsString;
use std::collections::HashMap;
use std::fs::{ReadDir, DirEntry, File};
use std::io::{Error, ErrorKind};
use std::process::Command;
use std::ops::Deref;
use std::fmt::Debug;
use std::thread::panicking;

pub struct Benchmark {
    src_dir: PathBuf,
    benchmark_name: OsString
}

const BINARY_DIR: &str = "./benchmarks/bin";
const OBJECT_DIR: &str = "./benchmarks/objects";
const LIBRARY_DIR: &str = "./allocators/target";
const BENCHMARK_DIR: &str = "./benchmarks/sources";
const COMMON_DIR: &str = "common";

impl Benchmark {
    pub fn new<P : Deref<Target=Path> + Debug>(path: P) -> Self {
        if !path.exists() {
            panic!("{:?} is a not real benchmark", path);
        }
        if !path.is_dir() {
            panic!("{:?} must be a directory", path);
        }
        let name = path.file_name().unwrap().to_os_string();
        Self {
            src_dir: path.to_path_buf(),
            benchmark_name: name
        }
    }

    /// Creates the directory for which objects are placed
    fn create_objects_dir() {
        let dir = Path::new(OBJECT_DIR);
        if !dir.exists() {
            std::fs::create_dir_all(dir).expect("Failed to create objects dir");
        }
    }

    /// Creates the directory where final benchmarks are created
    fn create_bin_dir() {
        let dir = Path::new(BINARY_DIR);
        if !dir.exists() {
            std::fs::create_dir_all(dir).expect("Failed to create binaries dir");
        }
    }

    fn clean_folder(folder: &Path) {
        let dir = std::fs::read_dir(folder).expect("Not a folder");
        for entry in dir {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_dir() {
                        Self::clean_folder(&*path);
                    } else {
                        let _ = std::fs::remove_file(path);
                    }
                },
                Err(_) => {},
            }
        }
        std::fs::remove_dir(folder);
    }

    fn clean_benchmarks() {
        let dir = Path::new(BINARY_DIR);
        if dir.exists() && dir.is_dir() {
            Self::clean_folder(dir)
        }
        let dir = Path::new(OBJECT_DIR);
        if dir.exists() && dir.is_dir() {
            Self::clean_folder(dir)
        }
    }


    /// Returns a mapping of available benchmarks to run
    pub fn available_benchmarks() -> Result<HashMap<OsString, Vec<PathBuf>>, std::io::Error> {
        let dir = std::fs::read_dir(Path::new(OBJECT_DIR))?;
        let mut output = HashMap::new();

        for benchmark_folder in dir {
            match benchmark_folder {
                Ok(benchmark_folder) => {
                    let name = benchmark_folder.file_name();
                    let read = std::fs::read_dir(benchmark_folder.path())?;
                    let binaries = read.filter_map(
                        |entry_result|  entry_result.ok()
                    ).map(
                        |entry| entry.path().to_path_buf()
                    ).collect();
                    output.insert(name, binaries);
                },
                Err(_) => {},
            }
        }


        Ok(output)
    }

    fn get_object_file(&self) -> OsString {
        OsString::from(format!("{}.o", self.benchmark_name.clone().into_string().unwrap()))
    }

    /// Creates an object file that has not been linked to an allocator yet
    ///
    /// Returns an error if it could not successfully create the object files
    pub fn create_object_file(&self) -> Result<(), std::io::Error> {
        Self::create_objects_dir();

        // Runs the make file in the benchmark folder
        if !Command::new("make")
            .current_dir(&self.src_dir)
            .status()?
            .success() {
            panic!("Failed to create object file for {:?}", self.benchmark_name)
        }

        // Create the object file
        let mut origin = self.src_dir.clone();
        origin.push(Path::new(&self.get_object_file()));

        // Move the object file to the objects folder
        if !Command::new("mv")
            .arg(origin)
            .arg(OBJECT_DIR)
            .status()?
            .success() {
            panic!("Failed to move file");
        }

        Ok(())
    }

    /// Creates all of the benchmark binaries for each allocator
    ///
    /// Returns an error if it could not successfully create the binary files
    pub fn create_binaries_for(self, allocators: &Vec<Option<String>>) -> Result<(), std::io::Error> {
        Self::create_bin_dir();

        let object_file = {
            let mut path = PathBuf::from(OBJECT_DIR);
            path.push(self.get_object_file());
            path
        };

        if !object_file.exists() {
            return Err(std::io::Error::last_os_error())
        }

        for allocator in allocators {
            let mut build = cc::Build::new();
            build.object(&object_file);
            build.static_flag(true);
            if let Some(allocator) = allocator {
                build.flag(format!("-L{}", LIBRARY_DIR).as_str());
                build.flag(format!("-l{}", allocator).as_str());
            }
            let allocator = allocator.as_ref().map_or(String::from("libc"), |a| a.clone());

            let output = format!("{}-{}", self.benchmark_name.to_str().unwrap(), allocator);
            let mut output_path = PathBuf::from(BINARY_DIR);
            output_path.push(output);
            build.compile(output_path.to_str().unwrap());
        }

        Ok(())
    }
}

pub fn get_available_benchmarks() -> Result<Vec<OsString>, std::io::Error> {
    let entries = std::fs::read_dir(BENCHMARK_DIR)?;
    let mut output = vec![];
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap();
                if name != COMMON_DIR {
                    output.push(path.into_os_string())
                }
            }
        }
    }
    Ok(output)
}

