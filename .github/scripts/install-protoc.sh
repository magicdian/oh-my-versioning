#!/usr/bin/env bash
set -euo pipefail

if command -v protoc >/dev/null 2>&1; then
  protoc --version
  exit 0
fi

run_as_root() {
  if command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    "$@"
  fi
}

case "${RUNNER_OS:-$(uname -s)}" in
  Linux)
    if command -v apt-get >/dev/null 2>&1; then
      run_as_root apt-get update
      run_as_root apt-get install --yes protobuf-compiler
    elif command -v dnf >/dev/null 2>&1; then
      run_as_root dnf install --assumeyes protobuf-compiler
    elif command -v yum >/dev/null 2>&1; then
      run_as_root yum install --assumeyes protobuf-compiler
    elif command -v apk >/dev/null 2>&1; then
      run_as_root apk add protobuf
    else
      echo "unsupported Linux package manager for protoc installation" >&2
      exit 1
    fi
    ;;
  macOS | Darwin)
    brew install protobuf
    ;;
  Windows | MINGW* | MSYS* | CYGWIN*)
    choco install protoc --yes --no-progress
    export PATH="/c/ProgramData/chocolatey/bin:$PATH"
    echo "C:\\ProgramData\\chocolatey\\bin" >> "$GITHUB_PATH"
    ;;
  *)
    echo "unsupported runner OS for protoc installation: ${RUNNER_OS:-$(uname -s)}" >&2
    exit 1
    ;;
esac

protoc --version
