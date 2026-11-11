{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShell = with pkgs;
          mkShell {
            buildInputs = [
              rustup
              pkg-config
              libudev-zero
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXrandr
              wayland
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath([
              libxkbcommon
              vulkan-loader
            ]);
          };
      });
}
