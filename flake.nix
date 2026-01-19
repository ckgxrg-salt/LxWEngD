{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
      };
    in
    {
      packages.${system}.default = pkgs.callPackage ./package.nix { };

      devShells.${system}.default = pkgs.mkShell {
        name = "lxwengd";

        buildInputs = with pkgs.xorg; [
          libxcb
          libXrandr
        ];

        nativeBuildInputs = with pkgs; [
          rust-analyzer
          cargo
          rustc
          clippy
          rustfmt
          deadnix
          nixfmt
        ];
      };
    };
}
