{
  description = "Read data from SwitchBot Meter Plus devices";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system: {
      devShell =
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        pkgs.mkShell {
          packages = with pkgs; [
            bluez
            cargo
	    clippy
            dbus
            gcc
            pkg-config
	    rustfmt
          ];
        };
    });
}
