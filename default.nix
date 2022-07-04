{ bluez, dbus, pkg-config, rustPlatform }:
let cargoTOML = with builtins; fromTOML (readFile ./src/meterreader/Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoTOML.package.name;
  version = cargoTOML.package.version;
  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  preBuildPhases = [ "codeStyleConformanceCheck" ];

  codeStyleConformanceCheck = ''
    header "Checking Rust code formatting"
    cargo fmt -- --check

    header "Running clippy"
    # clippy - use same checkType as check-phase to avoid double building
    if [ "''${cargoCheckType}" != "debug" ]; then
        cargoCheckProfileFlag="--''${cargoCheckType}"
    fi
    argstr="''${cargoCheckProfileFlag} --workspace --all-features --tests "
    cargo clippy -j $NIX_BUILD_CORES \
       $argstr -- \
       -D clippy::pedantic \
       -D warnings
  '';

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    bluez
    dbus
  ];
}
