{ # Use master branch of the overlay by default
  mozillaOverlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz),
  rustManifest ? ./channel-rust-nightly.toml,
}:

let
  pkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
in
with pkgs;
let
  rustcSrc = pkgs.fetchgit {
    url = https://github.com/rust-lang/rust.git;
    # master of 2019-09-25
    rev = "37538aa1365d1f8a10770a7d15c95b3167c8db57";
    sha256 = "1nvddkxwvrsvyx187s5mwj4fwsf26xd4vr6ba1kfy7m2fj7w79hq";
    fetchSubmodules = true;
  };
  target = "thumbv7em-none-eabihf";
  targets = [ target ];
  rustChannelOfTargets = _channel: _date: targets:
    (pkgs.lib.rustLib.fromManifestFile rustManifest {
      inherit (pkgs) stdenv fetchurl patchelf;
    }).rust.override { inherit targets; };
  rust =
    rustChannelOfTargets "nightly" null targets;
  rustPlatform = recurseIntoAttrs (makeRustPlatform {
    rustc = rust // { src = rustcSrc; };
    cargo = rust;
  });
  gcc = pkgsCross.armv7l-hf-multiplatform.buildPackages.gcc;
  xbuildRustPackage = attrs:
    let
      buildPkg = rustPlatform.buildRustPackage attrs;
    in
    buildPkg.overrideAttrs ({ name, nativeBuildInputs, ... }: {
      nativeBuildInputs =
        nativeBuildInputs ++ [ cargo-xbuild ];
      buildPhase = ''
        cargo xbuild --release --frozen
      '';
      XARGO_RUST_SRC = "${rustcSrc}/src";
      installPhase = ''
        mkdir $out
        cp target/${target}/release/${name} $out/${name}.elf
      '';
    });
  firmware = xbuildRustPackage {
    name = "firmware";
    src = ./firmware;
    cargoSha256 = "13nk3m9s7fy4anl89x5q88b7iar9y48ricj3k5ap741g2cll02dv";
    nativeBuildInputs = [
      gcc
    ];
    "CC_${target}" = "${gcc}/bin/armv7l-unknown-linux-gnueabihf-gcc";
    RUST_COMPILER_RT_ROOT = "${rustcSrc}/src/llvm-project/compiler-rt";
    doCheck = false;
  };
in {
  inherit pkgs rustPlatform rustcSrc gcc firmware;
}
