#!/usr/bin/env nu
with-env { CROSS_CONTAINER_OPTS: "--platform=linux/amd64" } {
    cross build --release --target armv7-unknown-linux-gnueabihf --target-dir ($env.PWD | path join zcross)
}


scp -r ./zcross/armv7-unknown-linux-gnueabihf/release/redpitaya ./static root@rp-f0bbb6.local:/root/
