{ lib, rustPlatform }:
# LxWEngD package
rustPlatform.buildRustPackage {
  pname = "lxwengd";
  version = "0.1.2";

  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    description = "A daemon that adds playlists to linux-wallpaperengine";
    homepage = "https://github.com/ckgxrg-salt/LxWEngD";
    license = lib.licenses.bsd2;
    maintainers = [ lib.maintainers.ckgxrg ];
  };
}
