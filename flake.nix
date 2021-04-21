{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
    ske.url = "git+ssh://git@github.com/aeronautical-informatics/ske?ref=main";
    ske-rs.url = "github:aeronautical-informatics/ske-rs";
  };

  outputs = { self, nixpkgs, utils, naersk, ske, ske-rs }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages."${system}";
      naersk-lib = naersk.lib."${system}";
      _ske = ske.defaultPackage."${system}";
      _ske-rs = ske-rs.defaultPackage."${system}";
    in rec {
      # `nix build`
      packages.rust-partitions = with pkgs; naersk-lib.buildPackage {
        pname = "rust-partitions";
        root = ./.;
        doCheck = true;
        doDoc = true;
        copyLibs = true;
        doDocFail = true;
        #override = _: { dontFixup = true; };
      };
      packages.ske-configuration = with pkgs; stdenv.mkDerivation {
        SKE_PATH = "${_ske}";
        PARTITIONS_ROOT = "${packages.rust-partitions}/lib";

        name = "ske-configuration";
        src = ./.;

        doCheck = true;
        checkInputs = [ _ske-rs ];

        installPhase = ''
          mkdir $out/
          mv *.elf *.xml features rtlola $out/
        '';

        dontFixup = true;
      };

      defaultPackage = packages.ske-configuration;

      # `nix develop`
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [ rustc cargo ];
      };

      # Hail to the Hydra
      hydraJobs = packages.ske-configuration;
    });
}
