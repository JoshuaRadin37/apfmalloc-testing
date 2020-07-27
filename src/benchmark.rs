use std::path::{Path, PathBuf};
use std::ffi::OsString;

pub struct Benchmark {
    src_dir: PathBuf,
    benchmark_name: OsString
}

impl Benchmark {
    pub fn new(path: &Path) -> Self {
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
        let dir = Path::new("./benchmarks/objects");
        if !dir.exists() {
            std::fs::create_dir_all(dir).expect("Failed to create objects dir");
        }
    }

    /// Creates an object file that has not been linked to an allocator yet
    pub fn create_object_file(&self) -> bool {
        Self::create_objects_dir();





        true
    }
}

