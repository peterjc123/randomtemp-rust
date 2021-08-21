# RandomTemp-Rust
A utility to override the temp directory for an executable, which is written in Rust.

## Why do we need this?
Sometimes, the executables use `TMP` and `TEMP` for finding the temporary directory. However, when you run them in parallel, it is possible that they will write to a file at the same time. We want to avoid this situation, so this utility is written.

## How to use this?
Simple usage 
### Windows
```cmd
set RANDOMTEMP_EXECUTABLE=set
./randomtemp.exe
:: you may notice that the variables are overriden
```
### Linux
```bash
export RANDOMTEMP_EXECUTABLE=set
./randomtemp.exe
# you may notice that the variables are overriden
```

More complicated case:
### Windows
```cmd
set RANDOMTEMP_EXECUTABLE=nvcc
./randomtemp.exe -v -ccbin ^
    "C:\Program Files (x86)\Microsoft Visual Studio\2017\Enterprise\VC\Tools\MSVC\14.11.25503\bin\HostX64\x64\cl.exe" ^
    "test.cpp"
:: you may notice that the generated temp directories are used
```
### Linux
```bash
export RANDOMTEMP_EXECUTABLE=nvcc
./randomtemp.exe -v -ccbin ^
    "/usr/bin/gcc-5" ^
    "test.cpp"
# you may notice that the generated temp directories are used
```

As an alternative to `RANDOMTEMP_EXECUTABLE`, you can also can pass the executable as the first argument
```bash
./randomtemp.exe nvcc -v -ccbin "/usr/bin/gcc-5" "test.cpp"
```

## Environment variables
- RANDOMTEMP_EXECUTABLE: target executable to be patched (Optional, default: name of the current executable)
- RANDOMTEMP_BASEDIR: directory to store temporary files (Optional, default: current working directory)
- RANDOMTEMP_MAXTRIAL: max times for retry the command (Optional, default: 3)

## How to build this?
Dependencies:
- Visual Studio 2017 / 2019 (Windows only)
- Clang / GCC (Linux only)
- Rust 1.42.0

You may run the following commands to build and test the project.
```bash
cargo build
# On Windows
set PATH=%PATH%;%CD%\target\build
# On Linux
export PATH="$PATH:$PWD/target/build"
cargo test
```

## Limitations
1. If you don't specify `RANDOMTEMP_EXECUTABLE`, the target executable may not be found under some circumstances.
