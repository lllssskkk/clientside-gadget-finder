{
  description = "ghunter adapter to work with chromium";

  inputs = {
    nixpkgs.url = "github:Nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    chromium4ghunter.url = "git+ssh://git@github.com/lllssskkk/chromium4ghunter.git";
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
              --url "https://smallforbig.com" "$@"
          '';
        };

      in
      {
        devShells.default = devShell;
        packages.default = runScript;
        apps.default = flake-utils.lib.mkApp { drv = runScript; };
        # packages = {
        #   default = runScript; # <-- must exist
        #   chromium-ghunter = chromium-ghunter.chromium-ghunter;
        #   chromium-ghunter-unwrapped = chromium-ghunter.chromium-ghunter-unwrapped;
        # };
      }
    );
}
# let
#   allSystems = ["x86_64-linux" "aarch64-linux" "i686-linux"];

#   genPackages = system: let
#     pkgs = import nixpkgs {
#       inherit system;
#     };
#     inherit (pkgs) lib;
#     inherit (lib) nameValuePair;
#   in
#     nameValuePair system (let
#       chromium-ghunter = pkgs.callPackage ./nix/chromium-ghunter {};
#     in {
#       inherit (chromium-ghunter) chromium-ghunter chromium-ghunter-unwrapped;
#       inherit pkgs;
#     });

#   genDevShells = system: let
#     inherit (selfPkgs) pkgs;
#     selfPkgs = self.packages.${system};
#   in
#     pkgs.lib.nameValuePair system {
#       default = pkgs.mkShell {
#         buildInputs = with pkgs; [
#           cargo
#           rustc
#           rustfmt
#           rust-analyzer
#           clippy

#           selfPkgs.chromium-ghunter
#         ];
#       };
#     };
# in {
#   # only x86_64-linux has been tested
#   packages = builtins.listToAttrs (map genPackages allSystems);
#   devShells = builtins.listToAttrs (map genDevShells allSystems);
# };
