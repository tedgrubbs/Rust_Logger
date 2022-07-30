#!/bin/bash
cargo build
cp target/debug/log log
sudo chown root log
sudo chmod u+s log
# ./target/debug/log -c "mpirun -np 4 lmp < /home/tedwing/Desktop/lammps/examples/crack/in.crack"
