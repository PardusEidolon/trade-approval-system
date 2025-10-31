{ pkgs, config, ... }:

{
  packages = with pkgs; [ cargo-nextest ];

  languages.rust.enable = true;

  outputs.rust-lib = config.languages.rust.import ./. { };

}
