#!/bin/bash
cargo build

EXEC_LOC=target/debug/tls_server

sudo chown root $EXEC_LOC
sudo chmod u+s $EXEC_LOC

$EXEC_LOC