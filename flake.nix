{
  description = "Shield for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };

        runtimeDependencies = with pkgs; [
          wayland
          libGL
          vulkan-loader
        ];

      in {
        devShells.default = with pkgs; mkShell rec {
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            wayland.dev
            libxkbcommon
            udev
            seatd
            libinput
            libgbm
            pixman
          ];

          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          RUSTFLAGS = "-C link-arg=-Wl,-rpath,${pkgs.lib.makeLibraryPath buildInputs}";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeDependencies;
        };
      }
    );
}


