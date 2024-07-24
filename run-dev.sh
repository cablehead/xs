#!/usr/bin/env bash

cargo b && { rm -f ./store/sock; ./target/debug/xs ./store; }
