let
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
in
with pkgs;
let
  project = callPackage ./default.nix {};
in
with project;
stdenv.mkDerivation {
  name = "armdev-env";
  buildInputs = with rustPlatform.rust; [
    rustc cargo cargo-xbuild
    rustcSrc
    pkgsCross.arm-embedded.stdenv.cc
    openocd
  ];

  # Set Environment Variables
  RUST_BACKTRACE = 1;
  XARGO_RUST_SRC = "${rustcSrc}/src";
  RUST_COMPILER_RT_ROOT = "${rustcSrc}/src/llvm-project/compiler-rt";

  shellHook = ''
    echo "Run 'cargo xbuild --release' to build."
  '';
}
