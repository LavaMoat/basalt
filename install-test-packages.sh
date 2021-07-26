#!/usr/bin/env bash

set -e;

for file in ./tests/mock-packages/*; do
  if test -d "$file"; then
    name=$(basename "$file");
    yarn add file:$file
  fi
done
