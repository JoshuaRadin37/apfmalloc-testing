# Memory Allocator Testing Platform
#### By Joshua Radin

## Description
This is a testing platform to run benchmarks on different memory allocators.
This platform will produce graphs per benchmark, comparing the results of the different
allocators. The measurement is the frequency at which the benchmarks can be
run.

## How to run
This is a rust project that utilizes cargo to function well. The basic
structure of this program is split into the 2 parts: The allocators and
the benchmarks.

### Benchmarks
Available benchmarks are found in the `benchmarks/sources` folder. To
add more benchmarks, all that is needed is to add another benchmark source
directory into this folder. The only requirement is that the benchmark folder
includes a makefile. The names of the benchmarks are automatically determined
by the name of the directory.

The currently available benchmarks are:

- `t-test1`
- `t-test2`

### Allocators
The available allocators are stored in the `allocators` folder. Adding new
allocators is a much more involved process than adding benchmarks, as
the program itself requires more knowledge than just the name of the allocator.
For example, of the 4 available allocators, 1 is written in C, 1 in C++, 1 in Rust,
and the final one (libc) isn't an extra compiled source, and instead relies on
the included allocator.

The currently available allocators are:
- `libc` - the included allocator
- `jemalloc` - the Google allocator
- `lrmalloc` - a lock-free allocator designed by Ricardo Leite and Wentao Cai
- `apfmalloc` - a lock-free allocator this platform was designed to test. It is the 
`lrmalloc` reimplemented in Rust, with the addition of APF tuning. This project was
written by Joshua Radin and Elias Neuman-Donihue.


### APF Tuning Environment Variables

There are several environment variables that can be set to affect the APF tuner. These are:

    TARGET_APF - The target APF for the allocator (Default: 2500)
    BURST_LENGTH - The length of traces to use during bursts (Default: 300)
    HIBERNATION_PERIOD - The time after a burst period where the APF tuner is inactive (Default: 2*BURST_LENGTH)


### Command Line Interface

USAGE:
    
- Stand alone binary: `lrmalloc-rs-testing [FLAGS] [OPTIONS] [SUBCOMMAND]`
- Through cargo: `cargo run -- [FLAGS] [OPTIONS] [SUBCOMMAND]`

FLAGS:
    
    -d, --debug      Generate debug symbols in output
        --dynamic    Use dynamic libraries instead of static
    -h, --help       Prints help information
    -v, --verbose    Shows verbose output
    -V, --version    Prints version information

OPTIONS:
    
    -a, --allocator <allocator>...    The allocator(s) to test. If no allocators are specified, all are tested
    -b, --benchmark <benchmark>...    The benchmarks to test. If not benchmarks are specified, all are run
        --features <features>...         Set features for the apfmalloc build (track_allocation, no_met_stack)
    -t, --threads <threads>           The maximum number of threads to test [default: 16]

SUBCOMMANDS:
    
    clean    Cleans the allocators, forcing a remake of the allocators
    help     Prints this message or the help of the given subcommand(s)


## Dynamic vs Static

Depending on the operating system, it may be better to run the benchmarks
using dynamic libraries or static libraries. The platform handles this difference
automatically.

Recommendations for OSs:
- Linux: Dynamic
    - Doesn't easily support static compilation
- MacOs: Static
- Windows: Unsupported

## General Flow of the Program

After the allocators and benchmarks are selected, which all for both by default,
binaries are produced for each combination of benchmark and allocator.

> When using *dynamic* libraries, the individual binaries are still produced for easy
> re-usability of code, despite the binaries all being exactly the same.

These binaries are stored in the `benchmarks/bin` folder. Then, the platform runs
each binary several times to find an average throughput for each number of threads from
1 to the number specified by the `-t` or `--threads` option. The default value for this
is 16.

Then, the platform produces a graph showing the difference between the selected allocators
for each benchmark. The results are stored for each run in the `graphs` folder. The result text
for the most recent run is also stored in the `benchmarks/results` folder.
