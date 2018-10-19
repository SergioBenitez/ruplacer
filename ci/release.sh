#!/bin/bash


if [[ -z $TRAVIS_TAG ]]; then
  echo "not on a tag, skipping release"
  exit 0
fi

if [[ $TRAVIS_RUST_VERSION != "stable" ]]; then
  echo "not building for stable rust, exiting"
  exit 0
fi

set -x
set -e

run_python3() {
  case $TRAVIS_OS_NAME in
    linux*)
      run_python3_linux "$@"
      ;;
    osx*)
      run_python3_osx "$@"
      ;;
    windows*)
      run_python3_windows "$@"
      ;;
    esac
}

run_python3_linux() {
  python3 "$@"
}

run_python3_osx() {
  /usr/local/bin/python3 $@
}

run_python3_windows() {
  /c/Python37/python $@
}

install_release_deps() {
  case $TRAVIS_OS_NAME in
    linux*)
      install_release_deps_linux
      ;;
    osx*)
      install_release_deps_osx
      ;;
    windows*)
      install_release_deps_windows
      ;;
    esac
}

install_release_deps_linux() {
  # travis uses an old version of python3 that
  # does not comes with pip
  curl https://bootstrap.pypa.io/get-pip.py -o get-pip.py
  python3 get-pip.py --user
}

install_release_deps_osx() {
  brew update
  (
    cd ci
    brew bundle install
  )
}

install_release_deps_windows() {
   choco install python
}

main() {
  install_release_deps

  run_python3 -m pip install github3.py path-py --user
  run_python3 ci/release.py --tag $TRAVIS_TAG --platform $TRAVIS_OS_NAME
}

main
