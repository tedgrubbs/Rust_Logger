#!/bin/bash

# This script is for main developers only. 
# It uploads new builds to the hosting server. 
# It will fail unless you have the a valid ssh key or the server password.

upload_location=ted@taylorgrubbs.online:/var/www/html/downloads/

# build log_client
echo -e "\nBuilding log..."
./make.sh log

# build log_server
echo -e "\nBuilding server..."
./make.sh log_server

# build pdf readme
echo -e "\nBuilding README pdf..."
pandoc -s -o README.pdf README.md

echo -e "\nUploading..."
# Place build info into html template 
server_build_info=`cat log_server/build/build.txt`
log_build_info=`cat log/build/build.txt`

# the insane stuff here is for removing the / from the sha output. So that sed can work
sed "s/<SERVER_BUILD_INFO>/${server_build_info//[\/]/\_}/" template.html > index.html
sed -i "s/<LOG_BUILD_INFO>/${log_build_info//[\/]/\_}/" index.html 

# then just need to upload these to the /download directory of the site
scp log_server/build/log_server log_server/log_server_installer.sh log/build/log log/log_installer.sh README.pdf LICENSE index.html $upload_location

# cleaning up 
rm index.html README.pdf
