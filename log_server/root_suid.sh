#!/bin/bash
cargo build

EXEC_LOC=target/debug/log_server

sudo chown root $EXEC_LOC
sudo chmod u+s $EXEC_LOC

$EXEC_LOC
