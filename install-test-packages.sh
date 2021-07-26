#!/usr/bin/env bash

set -e;

DIRS=$(find tests -name 'package.json');

for pkg in $DIRS; do
  file=$(dirname "$pkg");
  if test -d "$file"; then
    name=$(basename "$file");
    (cd "$file" && yarn link)
    yarn link $name
  fi
done
