#!/bin/bash

upload_location=ted@taylorgrubbs.online:/var/www/html/downloads/

server_build_info=`cat log_server/build/build.txt`
log_build_info=`cat log_client/build/build.txt`

# the insane stuff here is for removing the / from the sha output. So that sed can work
sed "s/<SERVER_BUILD_INFO>/${server_build_info//[\/]/\_}/" template.html > index.html
sed -i "s/<LOG_BUILD_INFO>/${log_build_info//[\/]/\_}/" index.html 

# then just need to upload these to the /download directory of the site
scp log_server/build/tls_server log_client/build/log index.html $upload_location

rm index.html
