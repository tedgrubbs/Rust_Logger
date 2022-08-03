#!/bin/bash

mkdir $HOME/.log
config='Username:Moscow\nServer:localhost:1241'

config_path=$HOME/.log/config

# Don't overwrite old config
if [ ! -f "$config_path" ]; then
  echo -e $config > $config_path
fi

sudo cp log /usr/bin/
sudo chown root /usr/bin/log
sudo chmod u+s /usr/bin/log


echo '### \nLog installation complete\n ###'