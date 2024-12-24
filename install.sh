#!/bin/sh

cargo build --release && \
sudo sh -c '
  cp target/release/smc_rw /usr/local/bin/ &&
  chown root /usr/local/bin/smc_rw &&
  chmod u+s /usr/local/bin/smc_rw '
