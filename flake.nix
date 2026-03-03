{
  description = "poziomki - Poznaj ciekawe osoby i spędzaj więcej czasu offline";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    treefmt-nix,
    git-hooks,
    naersk,
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

    getToolchain = pkgs:
      fenix.packages.${pkgs.stdenv.hostPlatform.system}.default.toolchain;
  in {
    formatter = eachSystem (pkgs: treefmtEval.${pkgs.stdenv.hostPlatform.system}.config.build.wrapper);

    checks = eachSystem (pkgs: {
      formatting = treefmtEval.${pkgs.stdenv.hostPlatform.system}.config.build.check self;
      pre-commit-check = git-hooks.lib.${pkgs.stdenv.hostPlatform.system}.run {
        src = ./.;
        hooks = {
          clippy.enable = true;
          shellcheck.enable = true;
          hadolint.enable = true;
        };
      };
    });

    packages = eachSystem (pkgs: rec {
      backend = let
        toolchain = getToolchain pkgs;
      in
        (naersk.lib.${pkgs.stdenv.hostPlatform.system}.override {
          cargo = toolchain;
          rustc = toolchain;
        }).buildPackage {
          src = ./backend;
          buildInputs = with pkgs; [
            pkg-config
            openssl
            libwebp
            libpq
          ];
        };

      default = backend;
    });

    devShells = eachSystem (pkgs: {
      default = pkgs.mkShell {
        buildInputs = with pkgs; [
          (getToolchain pkgs)
          pkg-config
          openssl
          libwebp
          libpq
        ];
      };
    });
  };
}
