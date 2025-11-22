#!/bin/bash
# Simple wrapper to run spaceway with Nix environment

set -e

# Just pass all arguments to spaceway inside nix shell
exec nix develop --command cargo +nightly run --bin spaceway -- "$@"
