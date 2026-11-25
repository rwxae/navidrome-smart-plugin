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

  cargoHash = "sha256-dV894w49zxaXBb63okg77+fuKW0JynNYgShF88EUTqM=";

  doCheck = true;

  meta = {
    description = manifest.description;
    homepage = manifest.homepage;
    license = lib.licenses.gpl3Only;
    sourceProvenance = with lib.sourceTypes; [ fromSource ];
  };
})
