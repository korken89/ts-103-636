{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # Provides a properly packaged Kani (cargo-kani built from source,
    # CBMC bundle patchelf'ed, KANI_HOME wired up) so `make verify`
    # works without `cargo kani setup` or an FHS wrapper. Follows our
    # nixpkgs: the fetchCargoVendor in their own (older) lock sends a
    # User-Agent that crates.io now rejects with 403.
    nix-tools = {
      url = "github:gleachkr/nix-tools";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    {
      nixpkgs,
      fenix,
      nix-tools,
      ...
    }:
    let
      pkgsFor =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              fenix.overlays.default
            ];
          };
        in
        {
          inherit system pkgs;

          # Fenix-managed Rust toolchain from rust-toolchain.toml.
          rustToolchain = pkgs.fenix.fromToolchainFile {
            dir = ./.;
            sha256 = "sha256-mvUGEOHYJpn3ikC5hckneuGixaC+yGrkMM/liDIDgoU=";
          };
        };

      # The nix-tools kani derivation only supports x86_64-linux.
      systems = [
        "x86_64-linux"
      ];

      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f (pkgsFor system));
    in
    {
      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt-rfc-style);

      devShells = forAllSystems (
        {
          system,
          pkgs,
          rustToolchain,
          ...
        }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              # VCS operations (e.g. in Makefile)
              git
              # Makefile is the single source of truth for all checks
              gnumake
              # Main Rust toolchain (incl. thumbv8m target, see
              # rust-toolchain.toml)
              rustToolchain
              # `make fuzz-smoke`
              cargo-fuzz
              # `make verify` (symbolic verification of the codecs)
              nix-tools.packages.${system}.kani
            ];
          };
        }
      );
    };
}
