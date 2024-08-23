#!/bin/bash

watchexec -c -r -w ./src -- 'rm -f ./store/sock && cargo r ./store'
