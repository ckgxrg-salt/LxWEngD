{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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

        buildInputs = with pkgs; [
          cargo
          rustc
          clippy
          rustfmt
          deadnix
          nixfmt-rfc-style
        ];
      };
    };
}
