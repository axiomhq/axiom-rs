with import <nixpkgs> {};

mkShell {
  nativeBuildInputs = with buildPackages; [
    rustup
    cargo-outdated
    cargo-edit
    cargo-license
  ];
}