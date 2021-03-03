FROM 192.168.2.36:5000/android-sdk

#install lsb-release for debian
RUN apt-get update \
     &&  apt-get install lsb-release -y \
     &&  rm -rf /var/lib/apt/lists/*

RUN echo 192.168.10.64 gitlab.justsafe.com >> /etc/hosts

WORKDIR /app

ADD config /app/config

ADD target/release/rust_build /app

ENTRYPOINT ["/app/rust_build"]