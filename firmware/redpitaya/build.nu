#!/usr/bin/env nu
with-env { CROSS_CONTAINER_OPTS: "--platform=linux/amd64" } {
    cross build --release --target armv7-unknown-linux-gnueabihf
}


scp ./target/armv7-unknown-linux-gnueabihf/release/redpitaya root@rp-f0bbb6.local:/root/

