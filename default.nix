{ # Use master branch of the overlay by default
  mozillaOverlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz),
}:

let
  pkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
in
with pkgs;
let
  rustcSrc = fetchgit {
    url = https://github.com/rust-lang/rust.git;
    # master of 2019-08-31
    rev = "b3146549abf25818fecfc7555f35358a948e27ad";
    sha256 = "1db3g1iq6ba5pdasffay1bpywdibv83z5nwp2dzi0fxvz5bqx1gi";
    fetchSubmodules = true;
  };
  targets = [
  ];
  rust =
    rustChannelOfTargets "nightly" null targets;
  rustPlatform = recurseIntoAttrs (makeRustPlatform {
    rustc = rust // { src = rustcSrc; };
    cargo = rust;
  });
in {
  inherit pkgs rustPlatform rustcSrc;
}
