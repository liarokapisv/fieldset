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
        stable.cargo
        stable.rustc
        stable.clippy
        stable.llvm-tools
        stable.rustfmt
        stable.rust-analyzer
        targets.thumbv7em-none-eabihf.stable.rust-std
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
              version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
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
                clang-tools
              ];
            };
          });
    };
}




