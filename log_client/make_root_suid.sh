#!/bin/bash
cargo build

EXEC_LOC=target/debug/log

sudo chown root $EXEC_LOC
sudo chmod u+s $EXEC_LOC

# ./log
$EXEC_LOC -c  $HOME/Desktop/lammps/examples/crack
# ./log mpirun -np 4 lmp -in $HOME/Desktop/lammps/examples/crack/in.crack
