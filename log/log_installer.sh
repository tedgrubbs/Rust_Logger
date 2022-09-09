#!/bin/bash

SERVER="https://taylorgrubbs.online/downloads"
INSTALL_DIR="/usr/bin"
EXECUTABLE="log"
CONFIG_DIR=".log"

# can specify -u to just uninstall
ARG1=${1:-null}

# will try to uninstall old version first
if [ -f "$INSTALL_DIR/$EXECUTABLE" ] || [ $ARG1 == "-u" ]; then
  echo -e "\nUninstalling older verion...\n"
  sudo rm /usr/bin/log
fi

# will stop here if you just want to uninstall
if [ $ARG1 == "-u" ]; then
  exit 0
fi

# grabbing executable from server
wget $SERVER/$EXECUTABLE

config='Username : tayg\nServer : localhost:1241\ntracked_files : in.'

config_path=$HOME/$CONFIG_DIR/config

# Don't overwrite old config
if [ ! -f "$config_path" ]; then
  mkdir $HOME/$CONFIG_DIR
  echo -e $config > $config_path
fi

# make executable suid binary
sudo cp $EXECUTABLE $INSTALL_DIR
sudo chown root $INSTALL_DIR/$EXECUTABLE
sudo chmod u+s $INSTALL_DIR/$EXECUTABLE
sudo chmod +x $INSTALL_DIR/$EXECUTABLE
rm $EXECUTABLE

echo -e '### Log installation complete ###'

