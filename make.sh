#!/bin/bash

# This just builds the release version of the program and stores it in a build/ directory with time and hash to loosely manage versioning

EXECUTABLE=$1
cd $EXECUTABLE
cargo build --release

OUTPUT_DIR="build"
mkdir $OUTPUT_DIR 2>/dev/null



# tries to move build to build directory. Stores any error messages
err=`mv target/release/$EXECUTABLE $OUTPUT_DIR/ 2>&1`
is_same=`echo $err | grep 'are the same file' | wc -l`
if [ $is_same == "1" ]; then
  echo "No change in build"
else
  tm=`date`
  hash=`sha256sum $OUTPUT_DIR/$EXECUTABLE`
  echo -e "<p>Build time: $tm<p>sha256: $hash" > $OUTPUT_DIR/build.txt
  echo -e "\nBuild complete\n"
  cat $OUTPUT_DIR/build.txt
fi

cd ..
