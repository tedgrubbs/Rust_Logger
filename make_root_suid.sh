#!/bin/bash
cargo build
sudo chown root target/debug/log
sudo chmod u+s target/debug/log
./target/debug/log 
