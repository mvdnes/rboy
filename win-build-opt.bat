cargo build
mkdir opt
rustc -O --out-dir opt -L target\deps src\rboy.rs
rustc -O --out-dir opt -L target\deps -L opt src\bin\rboy.rs
