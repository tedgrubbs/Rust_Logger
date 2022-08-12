#!/bin/bash

sudo systemctl disable log_server
sudo systemctl stop log_server
sudo rm /etc/systemd/system/log_server.service
sudo rm /usr/bin/tls_server