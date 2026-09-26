{
  description = "Personal command-line utilities";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    nixpkgs-darwin.url = "github:NixOS/nixpkgs/nixpkgs-26.05-darwin";
  };

  outputs =
    { nixpkgs, nixpkgs-darwin, ... }:
    let
      forEachSystem = nixpkgs.lib.genAttrs [
        "aarch64-darwin"
        "x86_64-linux"
      ];
      pkgsFor =
        system: (if system == "aarch64-darwin" then nixpkgs-darwin else nixpkgs).legacyPackages.${system};
    in
    {
      packages = forEachSystem (system: {
        git-smart-push = (pkgsFor system).callPackage ./python/git-smart-push/package.nix { };
      });

      checks = forEachSystem (
        system:
        let
          package = (pkgsFor system).callPackage ./python/git-smart-push/package.nix { };
        in
        {
          git-smart-push = package;
          git-smart-push-runtime = package.tests.runtime;
        }
      );
    };
}
