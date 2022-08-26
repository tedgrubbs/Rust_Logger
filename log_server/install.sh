#!/bin/bash

INSTALL_DIR="/usr/bin"
EXECUTABLE="tls_server"
CONFIG_DIR=".log_server"

# created this with cat fancytext | gzip | base64. where fancytext a file containing the og ascii art. Font is Delta Corps Priest 1
start_message="H4sIAAAAAAAAA61UOxbEIAjsPQVHtbCw2DoH9CS7b6MRA8ho4qOBgZGPGKgcqRyZ6jk1IckHZx653vAE7nn6GSGEYfBstJnfURUT2AwzgNeInlQW2ShVDDav1jSjgSviA6fxSPv4OtIpE4gZI5TBMila58/3L6LNpa8GEXex5E7V1NgZFPBywfJbJH1rM5VoGZDgPsE5oJSBaZ/6QLZrvX2PXpA5uNXbDYDXxt97WxK1PXHg0GG5SGskTj17mbm9uFLfP8784YAvhK4ntfIHAAA="

echo $start_message | base64 -d | gunzip
echo -e '\n'

echo -e '\n### Installing the log server ###\n'

# creating config file with default templates
config="server_port : 1241\ncert_path : $HOME/$CONFIG_DIR/myserver.crt\nkey_path : $HOME/$CONFIG_DIR/myserver.key\ndata_path : $HOME/$CONFIG_DIR/data/\ndatabase : LAMMPS"

config_path=$HOME/$CONFIG_DIR/config

# Don't overwrite old config
if [ ! -f "$config_path" ]; then

  # Creating directories for config and data uploads
  mkdir $HOME/$CONFIG_DIR
  mkdir $HOME/$CONFIG_DIR/data

  echo -e $config > $config_path

fi


# installing the executable
sudo cp build/$EXECUTABLE $INSTALL_DIR
sudo chown root $INSTALL_DIR/$EXECUTABLE
sudo chmod u+s $INSTALL_DIR/$EXECUTABLE

# building service
service_file="[Unit]\nDescription=Log Server service\nAfter=network.target\nStartLimitIntervalSec=0[Service]\nRestart=always\nRestartSec=1\n[Service]\nExecStart=$INSTALL_DIR/$EXECUTABLE\nType=simple\nUser=$USER\n[Install]\nWantedBy=multi-user.target\n"
echo -e $service_file > log_server.service
sudo mv log_server.service /etc/systemd/system/

# attempts starting server at install
sudo systemctl enable log_server
sudo systemctl start log_server

# checking status to see if successful
service_running=`ps aux | grep -v grep | grep $INSTALL_DIR/$EXECUTABLE | wc -l`

if [ $service_running != "0" ]; then
  echo -e "\nService started successfully"
else
  # print further installation instructions
  final_commands="\nTo start please set up config file at $HOME/$CONFIG_DIR/config. Check README.md for how to set config settings properly. Then run:\n\nsudo systemctl enable log_server\nsudo systemctl start log_server\n\nYou can check that it is running with: systemctl status log_server\nIf the above output says \"active (running)\" then the server installed successfully. :D "
  echo -e $final_commands
fi

echo -e '\n### Install complete. ###\n'

