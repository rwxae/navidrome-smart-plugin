{
  lib,
  buildNavidromeRustPlugin,
}:

let
  manifest = with builtins; (fromTOML (readFile ../Cargo.toml)).package;
in

buildNavidromeRustPlugin (finalAttrs: {
  pname = "smart-plugin";
  bundleName = "smart-plugin";

  version = manifest.version;

  src = ../.;

  cargoHash = "sha256-Vq5nYb0HKNAn3In9T9zgxCDp1PEOv45bT1pang29Suo=";

  doCheck = true;

  meta = {
    description = manifest.description;
    homepage = manifest.homepage;
    license = lib.licenses.gpl3Only;
    sourceProvenance = with lib.sourceTypes; [ fromSource ];
  };
})
