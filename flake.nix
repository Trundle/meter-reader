{
  description = "Read data from SwitchBot Meter Plus devices";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, flake-utils, nixpkgs, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;

          overlays = [
            (import rust-overlay)
          ];
        };

        meterRPackages = with pkgs.rPackages; [
          ggplot2
          ggthemes
          readr
          lubridate
          patchwork
        ];
      in
      {
        apps.plot-meter-data = flake-utils.lib.mkApp {
          drv = pkgs.writeShellApplication {
            name = "plot-meter-data";

            runtimeInputs = [
              (pkgs.rWrapper.override { packages = meterRPackages; })
            ];

            text = ''
              Rscript ${./src/plot.R} "$@"
            '';
          };
        };

        devShell = pkgs.mkShell {
          packages = with pkgs; [
            bluez
            cargo
            cargo-fuzz
            dbus
            gcc
            pkg-config
            rust-bin.nightly.latest.default

            (rstudioWrapper.override {
              packages = meterRPackages;
            })
          ];
        };
      });
}
