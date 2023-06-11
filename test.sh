#!/bin/bash

echo "HELP" | nc localhost 1337
echo "SIZE" | nc localhost 1337
echo "PX 5 5" | nc localhost 1337
echo "PX 5 5 ff8000" | nc localhost 1337
