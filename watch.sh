#!/usr/bin/env bash

cargo watch --why --ignore $(realpath ./store)'/*' -s "./run-dev.sh"
