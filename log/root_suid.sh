#!/bin/bash
cargo build

EXEC_LOC=/home/tedwing/Rust_Testing/Rust_Logger/log/target/debug/log

sudo chown root $EXEC_LOC
sudo chmod u+s $EXEC_LOC

# ./log
cd $HOME/Desktop/lammps/examples/crack
$EXEC_LOC  --coll newtest
# ./log mpirun -np 4 lmp -in $HOME/Desktop/lammps/examples/crack/in.crack
