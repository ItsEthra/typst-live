{
  inputs = {
    systems = {
      url = "github:nix-systems/default";
      flake = false;
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    systems,
    nixpkgs,
    crane,
  }: let
    inherit (nixpkgs) lib;
    eachSystem = f:
      lib.genAttrs (import systems) (system:
        f {
          inherit system;
          pkgs = nixpkgs.legacyPackages.${system};
        });
  in {
    packages = eachSystem ({
      system,
      pkgs,
    }: let
      craneLib = crane.lib.${system};
      cargoSource = lib.cleanSourceWith {
        src = craneLib.path ./.;
        filter = path: type:
          (craneLib.filterCargoSources path type)
          || (builtins.match ".*\\.html" path != null);
      };
    in {
      default = craneLib.buildPackage {
        src = cargoSource;
        buildInputs = with pkgs;
          [iconv]
          ++ lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreServices;
      };
    });

    checks = eachSystem ({
      system,
      pkgs,
    }: {
      default = pkgs.runCommand "typst-live-check" {} ''
        ${self.packages.${system}.default}/bin/typst-live --help > $out
        ${pkgs.gnugrep}/bin/grep typst-live $out
      '';
    });
  };
}
