#!/bin/bash

INSTALL_DIR="/usr/bin"
EXECUTABLE="log"
CONFIG_DIR=".log"

config='Username tayg\nServer localhost:1241'

config_path=$HOME/$CONFIG_DIR/config

# Don't overwrite old config
if [ ! -f "$config_path" ]; then
  mkdir $HOME/$CONFIG_DIR
  echo -e $config > $config_path
fi

# make executable suid binary
sudo cp build/$EXECUTABLE $INSTALL_DIR
sudo chown root $INSTALL_DIR/$EXECUTABLE
sudo chmod u+s $INSTALL_DIR/$EXECUTABLE

echo -e '### Log installation complete ###'
