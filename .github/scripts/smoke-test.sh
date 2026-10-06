#!/usr/bin/env bash
set -euo pipefail

tar xzf "grep-$TARGET.tar.gz"
if [[ "$GITHUB_REF_TYPE" == "tag" ]]; then
  expected_version="${GITHUB_REF_NAME#v}"
else
  expected_version=$(grep -m1 '^version = ' Cargo.toml | cut -d '"' -f2)
fi
test -n "$expected_version"

for binary in grep fastgrep; do
  "./$binary" --help
  version=$("./$binary" --version)
  printf '%s\n' "$version"
  printf '%s\n' "$version" | grep -F "grep (fastgrep) $expected_version ("
  result=$(printf 'hello world\n' | "./$binary" hello)
  test "$result" = 'hello world'
done
