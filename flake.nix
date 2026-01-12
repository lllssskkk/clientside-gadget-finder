{
  description = "ghunter adapter to work with chromium";

  inputs = {
    nixpkgs.url = "github:Nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    chromium4ghunter.url = "github:lllssskkk/chromium4ghunter.git";
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      chromium4ghunter,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        devEnv = (
          with pkgs;
          [
            cargo
            rustc
            rustfmt
            rust-analyzer
            clippy
          ]
        );

        flakeDep = [
          chromium4ghunter.packages.${system}.default
        ];

        project = pkgs.rustPlatform.buildRustPackage {
          pname = "ghunter4chrome";
          version = "0.1";

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "chromiumoxide-0.7.0" = "sha256-ZRTk5r5WLq9c+rvjqyAnB7pzdFbXl2IQBTg2w77IllY=";
            };

          };

          nativeBuildInputs = devEnv;

          src = pkgs.lib.cleanSource ./.;
        };
        devShell = pkgs.mkShell {
          packages = devEnv ++ flakeDep;
          shellHook = ''
            export PATH=${self}/bin:$PATH
          '';
        };

        # runScript = pkgs.writeShellApplication {
        #   name = "ghunter4chrome";
        #   runtimeInputs = [
        #     project

        #   ] ++ flakeDep;
        #   text = ''
        #     export PATH=${chromium4ghunter}/bin:$PATH
        #     export PATH=${project}/bin:$PATH
        #   '';
        # };
        runScript = pkgs.writeShellApplication {
          name = "ghunter4chromium";
          runtimeInputs = [
            project
            chromium4ghunter.packages.${system}.default
          ];

          text = ''
            exec ${project}/bin/ghunter4chromium-gadget-finder \
              --chromium-executable ${chromium4ghunter.packages.${system}.default}/bin/chromium-ghunter \
              --url "$@"
          '';
        };

      in
      {
        devShells.default = devShell;
        packages.default = runScript;
        apps.default = flake-utils.lib.mkApp { drv = runScript; };
      }
    );
}
