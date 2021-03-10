FROM 192.168.2.36:5000/android-sdk:update

#install lsb-release for debian
RUN apt-get update \
     &&  apt-get install lsb-release -y \
     &&  rm -rf /var/lib/apt/lists/*

COPY lib/ZKM.jar /lib/ZKM.jar

WORKDIR /app

ADD config /app/config

ADD target/release/rust_build /app

ENTRYPOINT ["/app/rust_build"]