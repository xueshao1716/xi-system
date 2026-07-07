#!/bin/bash
cd /mnt/d/xi-system
exec ./target/release/xi-system > /tmp/xi-out.log 2> /tmp/xi-err.log
