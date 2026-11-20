{
  lib,
  buildNavidromeRustPlugin,
}:

buildNavidromeRustPlugin (finalAttrs: {
  pname = "smart-plugin";
  bundleName = "smart-plugin";

  version = "0.1.0";

  src = ../.;

  cargoHash = "sha256-Vq5nYb0HKNAn3In9T9zgxCDp1PEOv45bT1pang29Suo=";

  doCheck = true;

  meta = {
    description = "TODO";
    homepage = "https://github.com/rwxae/navidrome-smart-plugin";
    license = lib.licenses.gpl3Only;
    sourceProvenance = with lib.sourceTypes; [ fromSource ];
  };
})
