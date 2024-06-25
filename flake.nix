{
  inputs = {
    nixpkgs = {
      url = "github:NixOs/nixpkgs/nixos-unstable";
    };
    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, fenix, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];

      buildToolchain = system: with fenix.packages.${system}; combine [
        latest.cargo
        latest.rustc
        latest.clippy
        latest.llvm-tools
        latest.rustfmt
        latest.rust-analyzer
      ];
    in
    {
      packages = forAllSystems
        (system:
          let
            pkgs = nixpkgs.legacyPackages.${system};
            toolchain = buildToolchain system;
            buildRustPackage = (pkgs.makeRustPlatform {
              cargo = toolchain;
              rustc = toolchain;
            }).buildRustPackage;
          in
          {
            default = buildRustPackage {
              pname = "fieldset";
              version = "0.1.0";
              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
              };
            };
          });

      devShells = forAllSystems
        (system:
          let
            pkgs = nixpkgs.legacyPackages.${system};
            toolchain = buildToolchain system;
          in
          {
            default = pkgs.mkShell {
              inputsFrom = [
                self.packages.${system}.default
              ];
              nativeBuildInputs = with pkgs; [
                toolchain
                cargo-expand
                cargo-binutils
                cargo-udeps
                cargo-readme
                clang-tools
              ];
            };
          });
    };
}




