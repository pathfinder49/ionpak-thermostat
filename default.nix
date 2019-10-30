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
    # master of 2019-10-30
    rev = "aa69777ea2902208b24b3fd77767d577ceaf6386";
    sha256 = "0aq9rb6g7g46abphbvgrig80yymdf75rhllf5pgygardqnh11a02";
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
    checkPhase = ''
      cargo test --target=${hostPlatform.config}
    '';
  };
in {
  inherit pkgs rustPlatform rustcSrc gcc firmware;
}
