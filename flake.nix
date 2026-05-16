{
  description = "TraceDrivenSimulator - RISC-V trace-driven simulator with C++17 and Spike";

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

            # C++17 toolchain
            cmake
            gtest
            gdb

            # IDE 支持 (clangd 语法解析)
            clang-tools

            # Task runner
            just

            # Useful development tools
            pkg-config
          ];

          # Spike requires RISC-V proxy kernel & bootloader
          SPIKE_PK = "${pkgs.spike}/share/spike/pk";

          shellHook = ''
            echo "🛠️  TraceDrivenSimulator dev shell"
            echo "   spike:  $(spike --version 2>&1 | head -1 || echo 'available')"
            echo "   g++:    $(g++ --version | head -1)"
            echo "   cmake:  $(cmake --version | head -1)"
          '';
        };
      }
    );
}
