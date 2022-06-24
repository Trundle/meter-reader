{
  description = "Read data from SwitchBot Meter Plus devices";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
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
            clippy
            dbus
            gcc
            pkg-config
            rustfmt

            (rstudioWrapper.override {
              packages = meterRPackages;
            })
          ];
        };
      });
}
