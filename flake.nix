{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.git-hooks.url = "github:cachix/git-hooks.nix";
  inputs.git-hooks.inputs.nixpkgs.follows = "nixpkgs";

  outputs =
    {
      self,
      nixpkgs,
      git-hooks,
    }:
    let
      eachSystem =
        fn:
        nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed (system: fn nixpkgs.legacyPackages.${system});
    in
    {
      checks = eachSystem (pkgs: {
        pre-commit-check = git-hooks.lib.${pkgs.stdenv.hostPlatform.system}.run {
          src = ./.;
          hooks = {
            nixfmt.enable = true;
            clippy.enable = true;
            rustfmt = {
              enable = true;
              settings.check = true;
            };
          };
        };
      });

      packages = eachSystem (pkgs: rec {
        navidrome-smart-plugin = pkgs.pkgsCross.wasi32.callPackage ./nix/package.nix { };
        default = navidrome-smart-plugin;
      });

      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          inherit (self.checks.${pkgs.stdenv.hostPlatform.system}.pre-commit-check) shellHook;
          packages = with pkgs; [
            cargo
            cargo-expand
            clippy
            rust-analyzer
            rustc
            rustfmt
          ];
        };
      });
    };
}
