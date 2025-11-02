{ pkgs, config, ... }:

{
  packages = with pkgs; [
  ];

  languages.rust.enable = true;

  outputs.rust-lib = config.languages.rust.import ./. { };

}
