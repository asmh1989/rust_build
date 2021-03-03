#!/bin/bash
pwd=`pwd`

#!/bin/bash
if [[ -n $(git diff --stat)  ]]
then
  echo '请先commit!!!'
  exit 1
else
  echo '代码已提交, 继续'
fi

version=$(date +"-%Y-%m-%d-$(git rev-parse --short HEAD)")

echo "build release version = ${version}"
sed -i "s/^version = \"\([0-9.]*\)\"/version = \"\1$version\"/g" Cargo.toml


echo " start build rust-build ..."
env OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu/ OPENSSL_INCLUDE_DIR=/usr/local/include OPENSSL_STATIC=yes cargo build --release
echo "build bmdm rust-build ..."

echo "build docker"
sudo docker build -t 192.168.2.36:5000/rust_build . 
echo "build docker end..."

sudo docker push 192.168.2.36:5000/rust_build:latest

git checkout -- Cargo.toml
git checkout -- Cargo.lock

