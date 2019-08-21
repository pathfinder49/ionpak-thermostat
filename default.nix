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
    # master of 2019-08-18
    rev = "ea52be482ab4945fda63cb65b6a198309a041e3c";
    sha256 = "1spifrkvyyrh1gazqrby29fjqsdbwvajv9k9f6mk2ldrdghlsd21";
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
