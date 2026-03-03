{
  description = "poziomki";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    fenix.url = "github:nix-community/fenix/monthly";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = {
    self,
    nixpkgs,
    treefmt-nix,
    git-hooks,
    fenix,
  }: let
    inherit (nixpkgs) legacyPackages lib;
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
      "i686-linux"
      "x86_64-darwin"
    ];

    eachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    treefmtEval = eachSystem (pkgs: treefmt-nix.lib.evalModule pkgs ./treefmt.nix);

    pkgsForEach = legacyPackages;
  in {
    formatter = eachSystem (pkgs: treefmtEval.${pkgs.system}.config.build.wrapper);
    checks = eachSystem (pkgs: {
      formatting = treefmtEval.${pkgs.system}.config.build.check self;
       pre-commit-check = git-hooks.lib.${pkgs.system}.run {
          src = ./.;
          hooks = {
            clippy.enable = true;
            shellcheck.enable = true;
            hadolint.enable = true;
          };
        };
    });
    devShells = eachSystem (pkgs: {
      default = pkgs.mkShell {
        buildInputs = with pkgs; [
          fenix.packages.${pkgs.system}.default.toolchain
        ];
      };
    });
  };
}
