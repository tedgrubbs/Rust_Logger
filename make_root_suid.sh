#!/bin/bash
cargo build
sudo chown root target/debug/log
sudo chmod u+s target/debug/log
./target/debug/log -c "mpirun -np 2 lmp -in /home/win4datay/Desktop/lammps/examples/crack/in.crack"
