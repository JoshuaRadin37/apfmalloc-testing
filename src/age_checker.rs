use std::ffi::OsString;
use crate::get_allocator_lib_file;
use std::fs::{ReadDir, DirEntry};
use std::time::SystemTime;
use std::io::Error;
use std::path::PathBuf;
use crate::benchmark::LIBRARY_DIR;
use std::ops::Not;

fn get_lib_file_name_actual(name: &str) -> String {
    format!("lib{}.a", get_allocator_lib_file(name).expect("Not a library name"))
}

fn last_modified_time(directory: ReadDir) -> Option<SystemTime> {
    let mut output = None;

    for entry in directory {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                let last_modified = if path.is_dir() {
                    let dir = std::fs::read_dir(path).unwrap();
                    last_modified_time(dir)
                } else {
                    let file = std::fs::metadata(path).unwrap();
                    Some(file.modified().unwrap())
                };
                match last_modified {
                    None => {},
                    Some(time) => {
                        match output {
                            None => {
                                output = Some(time);
                            },
                            Some(current_time) => {
                                if time > current_time {
                                    output = Some(time);
                                }
                            },
                        }
                    },
                }
            },
            Err(_) => {},
        }
    }

    output
}

pub fn should_build(name: &str) -> bool {
    let actual_lib_file = {
        let name = get_lib_file_name_actual(name.clone());
        let mut path = PathBuf::from(LIBRARY_DIR);
        path.push(name);
        path
    };
    if !actual_lib_file.exists() {
        return true;
    }
    let allocator_directory = std::fs::read_dir(PathBuf::from(format!("./allocators/{}", name))).unwrap();

    let library_modified_time = std::fs::metadata(actual_lib_file).unwrap().modified().unwrap();
    let directory_time = last_modified_time(allocator_directory);

    match directory_time {
        None => {
            true
        },
        Some(time) => {
            library_modified_time < time
        },
    }


}