
with (import <nixpkgs> {});
let
  libs = [
     pkg-config 
     cmake 
     openssl
];
in 
mkShell {
      packages = [ clang ];
      buildInputs = libs;
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libs;
}
