{ pkgs, config, ... }:

{
  packages = with pkgs; [
    cargo-nextest
    cargo-modules
  ];

  languages.rust.enable = true;

  outputs.rust-lib = config.languages.rust.import ./. { };

}
