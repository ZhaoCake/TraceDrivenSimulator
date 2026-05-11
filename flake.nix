{
  description = "TraceDrivenSimulator - RISC-V trace-driven simulator with Spike and Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # RISC-V ISA Simulator
            spike
            # RISC-V Cross Compiler
            pkgsCross.riscv32.buildPackages.gcc
            dtc

            # Rust toolchain
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer

            # Useful development tools
            pkg-config
          ];

          # Spike requires RISC-V proxy kernel & bootloader
          SPIKE_PK = "${pkgs.spike}/share/spike/pk";

          shellHook = ''
            echo "🛠️  TraceDrivenSimulator dev shell"
            echo "   spike:  $(spike --version 2>&1 | head -1 || echo 'available')"
            echo "   rustc:  $(rustc --version)"
            echo "   cargo:  $(cargo --version)"
          '';
        };
      }
    );
}
