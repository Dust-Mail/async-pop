{
  description = "An async Pop3 client for Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:NixOS/flake-compat";
      flake = false;
    };
  };

  outputs = {
    self,
    flake-utils,
    nixpkgs,
    rust-overlay,
    pre-commit-hooks,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];

      pkgs = import nixpkgs {
        inherit system overlays;
      };

      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-src" "clippy"];
      };

      extraPackages = [rust pkgs.pkg-config pkgs.openssl];
    in {
      # Run the hooks with `nix fmt`.
      formatter = let
        inherit (self.checks.${system}.pre-commit-check) config;
      in
        pkgs.writeShellScriptBin "pre-commit-run" ''
          ${pkgs.lib.getExe config.package} run --all-files --config ${config.configFile}
        '';

      checks = {
        pre-commit-check = pre-commit-hooks.lib.${system}.run {
          src = ./.;

          settings = {
            rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {
              lockFile = ./Cargo.lock;
            };
          };

          hooks = {
            # Adds extra packages needed for cargo and clippy
            extra-packages = {
              enable = true;

              inherit extraPackages;
              # necessary to have an entry
              entry = "true";
            };

            # Formatter for Nix code.
            alejandra.enable = true;

            cargo-check.enable = true;

            cargo-test = {
              enable = true;
              entry = "${pkgs.lib.getExe' rust "cargo"} test";
              files = "\\.rs$";
              pass_filenames = false;
            };

            clippy = {
              enable = true;

              packageOverrides = {
                clippy = rust;
                cargo = rust;
              };

              settings = {
                denyWarnings = true;
              };
            };

            # Check for Nix code that is unused.
            deadnix.enable = true;

            # Check if in-use version of nixpkgs is still maintained.
            flake-checker.enable = true;

            nil.enable = true;

            rustfmt.enable = true;

            statix.enable = true;
          };
        };
      };

      devShells = {
        default =
          pkgs.mkShell
          {
            # Create pre commit hooks upon entering the development shell
            inherit (self.checks.${system}.pre-commit-check) shellHook;

            buildInputs = self.checks.${system}.pre-commit-check.enabledPackages;

            packages = extraPackages;
          };
      };
    });
}
