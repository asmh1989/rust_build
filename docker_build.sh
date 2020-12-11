#!/bin/bash
pwd=`pwd`

echo " start build rust-build ..."
env OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/ OPENSSL_INCLUDE_DIR=/usr/local/include OPENSSL_STATIC=yes cargo build --release
echo "build bmdm rust-build ..."

echo "build docker"
sudo docker build -t 192.168.2.36:5000/rust_build . 
echo "build docker end..."

