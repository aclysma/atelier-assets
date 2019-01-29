@0xdbbcd8ccbd223b6c; # unique file ID, generated by `capnp id`

struct AssetUuid {
    id @0 :Data;
}

struct KeyValue {
  key @0 :Data;
  value @1 :Data;
}

enum FileType {
  file @0;
  directory @1;
  symlink @2;
  none @3;
}

enum FileState {
  exists @0;
  deleted @1;
}

enum AssetSource {
  file @0;
}

struct DirtyFileInfo {
  state @0 :FileState;
  sourceInfo @1 :SourceFileInfo;
}

struct SourceFileInfo {
  type @0 :FileType;
  lastModified @1 :UInt64;
  length @2 :UInt64;
}

struct RenameFileEvent {
  src @0 :Data;
  dst @1 :Data;
}

struct AssetUuidPair {
  key @0 :AssetUuid;
  value @1 :AssetUuid;
}

struct SourceMetadata {
  assets @0 :List(AssetUuid);
  importerVersion @1 :UInt32;
  importerOptionsType @2 :Data;
  importerOptions @3 :Data;
  importerStateType @4 :Data;
  importerState @5 :Data;
  buildPipelines @6 :List(AssetUuidPair);
}

struct ImportError {
  text @0 :Text;
}

# The identifier for an import artifact is the hash of 
# - Source asset contents
# - Source metadata contents
# - Importer version/id
struct ImportArtifactKey {
  hash @0 :Data;
}

struct AssetMetadata {
  id @0 :AssetUuid;
  loadDeps @1 :List(AssetUuid);
  buildDeps @2 :List(AssetUuid);
  instantiateDeps @3 :List(AssetUuid);
  searchTags @4 :List(KeyValue);
  buildPipeline @5 :AssetUuid;
}

struct ImportedMetadata {
  metadata @0 :AssetMetadata;
  # The most recently recorded hash of the input to the import function
  latestArtifact :union {
    id @1 :ImportArtifactKey;
    none @2 :Void;
  }
  # The source of the imported asset
  source @3 :AssetSource;
}

# The identifier for a build artifact is the hash of 
# - Import artifact hash
# - Build configuration (target platform etc)
struct BuildArtifactKey {
    hash @0 :Data;
}

struct BuildArtifact {
    key @0 :BuildArtifactKey;
    data @1 :Data;
}

struct BuildParameters {

}

