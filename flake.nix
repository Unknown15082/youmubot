{
  description = "A discord bot for Dự Tuyển Tổng Hợp server";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-22.05";
    nixpkgs-unstable.url = "github:nixos/nixpkgs";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, nixpkgs-unstable, naersk, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages."${system}";
      pkgs-unstable = nixpkgs-unstable.legacyPackages."${system}";
      naersk-lib = naersk.lib."${system}";
    in
    rec {
      packages.youmubot = naersk-lib.buildPackage {
        name = "youmubot";
        version = "0.1.0";

        root = ./.;
        cargoBuildOptions = opts: opts ++ [ "--package youmubot" ];

        nativeBuildInputs = nixpkgs.lib.optionals (nixpkgs.lib.strings.hasSuffix "linux" system) (with pkgs; [
          pkg-config
          openssl
        ]);
      };

      defaultPackage = packages.youmubot;

      # `nix run`
      apps.youmubot = flake-utils.lib.mkApp {
        drv = packages.youmubot;
        exePath = "/bin/youmubot";
      };
      defaultApp = apps.youmubot;

      # `nix develop`
      devShell = pkgs.mkShell
        {
          nativeBuildInputs =
            (with pkgs; [ rustc cargo ])
            ++ (with pkgs-unstable; [ rust-analyzer rustfmt ])
            ++ nixpkgs.lib.optionals (nixpkgs.lib.strings.hasSuffix "darwin" system) (with pkgs; [
              libiconv
              darwin.apple_sdk.frameworks.Security
            ])
            ++ nixpkgs.lib.optionals (nixpkgs.lib.strings.hasSuffix "linux" system) (with pkgs; [
              pkg-config
              openssl
            ]);
        };
      # module
      nixosModule = import ./module.nix defaultPackage;
    });
}

