#!/bin/bash
cargo build
cp target/debug/log log
sudo chown root log
sudo chmod u+s log
sudo cp log /usr/bin/
sudo chown root /usr/bin/log
sudo chmod u+s /usr/bin/log
./log
# ./log mpirun -np 4 lmp -in /home/win4datay/Desktop/lammps/examples/crack/in.crack
