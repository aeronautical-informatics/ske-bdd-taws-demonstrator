{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    ske.url = "git+ssh://git@github.com/aeronautical-informatics/ske?ref=main";
    ske-rs.url = "github:aeronautical-informatics/ske-rs";
  };

  outputs = { self, nixpkgs, utils, naersk, ... } @ inputs:
    utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";
        naersk-lib = naersk.lib."${system}";
        ske = inputs.ske.defaultPackage."${system}";
        ske-rs = inputs.ske-rs.defaultPackage."${system}";
      in
      rec {
        # `nix build`
        packages = utils.lib.flattenTree {
          partitions = with pkgs; naersk-lib.buildPackage {
            pname = "partitions";
            root = ./.;
            doCheck = true;
            #doDoc = true;
            copyLibs = true;
            #doDocFail = true;
            override = x: {
              C_INCLUDE_PATH = "${ske}/include";
              nativeBuildInputs = [ pkgs.rustPlatform.bindgenHook ] ++ x.nativeBuildInputs;
            };
          };
          ske-configuration = with pkgs; stdenv.mkDerivation {
            SKE_PATH = ske;
            PARTITIONS_ROOT = "${packages.partitions}/lib";

            name = "ske-configuration";
            src = ./.;

            doCheck = true;
            checkInputs = [ ske-rs ];

            installPhase = ''
              mkdir $out/
              mv *.elf *.xml features rtlola $out/
            '';

            dontFixup = true;
          };
        };

        defaultPackage = packages.ske-configuration;

        # `nix develop`
        devShell = pkgs.mkShell {
          SKE_PATH = ske;
          C_INCLUDE_PATH = "${ske}/include";
          inputsFrom = with packages; [ partitions ];
          nativeBuildInputs = [ ske ];
        };
      });
}
