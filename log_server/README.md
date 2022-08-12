# Log Server
#### This is under active development and may undergo significant changes. I will attempt to keep this up to date as development continues

## Installation

You can run the `install.sh` script to install the server. It is more convenient to run this as non-root, since it installs config files in your user home directory. This makes it easier to edit the config file.

This installs the `tls_server` executable to the `/usr/bin/` directory. It will also attempt to  start a service via systemctl- although it will probably fail unless your `config` is properly set up (see below). The service file is installed as `/etc/systemd/system/log_server.service`.

A new directory is made in the user's home folder called `.log_server/`. This stores a file called `config` and a subdirectory called `data/`. The `data/` directory is the location where file uploads are written.  

The `config` file contains a few configuration settings which are essentially key-value pairs separated by whitespace. Both keys and values are simple strings without quotation marks. An example config is shown below:  

>server_port 1241   
>cert_path /home/tedwing/.log_server/myserver.crt  
>key_path /home/tedwing/.log_server/myserver.key  
>data_path /home/tedwing/.log_server/data/  
>database LAMMPS  
>registry data_registry  
>tracked_files log. in.

`server_port`: Port where server should listen for incoming connections  
`cert_path`: path to crt file for TLS encryption  
`key_path`: path to private key file for TLS encryption  
`data_path`: path to where uploaded data is stored. Defaults to the `data/` directory mentioned above  
`database`: Name of MongoDB database where data is stored  
`registry`: Name of collection where record of files are first made. This just records the file names, hash, and contents as raw strings.  
`tracked_files`: List of file extensions to track. Log_Server will look for the appearance of these substrings in file names. If there is a match the file will be written to the database

Note that the TLS cert and key, if self-signed, will not be trusted by devices by default. You will either have to use a trusted certificate authority or manually add your cert to your systems' list of trusted certs. `Creating self-signed certificates.txt` explains how to install self-signed certs to a client device. I think in production though we should probably be using a cert from a trusted authority, otherwise we need to probably set up a root CA and make all devices trust it rather than manually adding certs to every machine.

