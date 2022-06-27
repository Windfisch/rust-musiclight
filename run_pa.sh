#!/bin/sh

parec --raw --rate 48000 --format=s16ne --channels=1 | target/release/musiclight
