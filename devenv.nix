{ pkgs, config, ... }:

{
  packages = with pkgs; [
  ];

  languages.rust.enable = true;
  languages.python = {
    enable = true;
    directory = "./python";
    uv = {
      enable = true;
    };
  };

  outputs.rust-lib = config.languages.rust.import ./. { };

}
