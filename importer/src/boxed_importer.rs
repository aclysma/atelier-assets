use crate::{error::Result, AsyncImporter, ExportAsset, ImporterValue, SerdeObj};
use atelier_core::{AssetRef, AssetTypeId, AssetUuid, CompressionType};
use erased_serde::Deserializer;
use futures_core::future::BoxFuture;
use futures_io::{AsyncRead, AsyncWrite};
use serde::{Deserialize, Serialize};
use type_uuid::{TypeUuid, TypeUuidDynamic};

/// Serializable metadata for an asset.
/// Stored in .meta files and metadata DB.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Default)]
pub struct AssetMetadata {
    /// UUID for the asset to uniquely identify it
    pub id: AssetUuid,
    /// Search tags are used by asset tooling to search for the imported asset
    pub search_tags: Vec<(String, Option<String>)>,
    /// The referenced build pipeline is invoked when a build artifact is requested for the imported asset
    pub build_pipeline: Option<AssetUuid>,
    /// The latest artifact produced when importing this asset
    pub artifact: Option<ArtifactMetadata>,
}

/// Serializable metadata for an artifact.
/// Stored in .meta files and metadata DB.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Default)]
pub struct ArtifactMetadata {
    /// Hash that identifies this artifact
    pub hash: u64,
    /// UUID for this artifact's asset
    pub id: AssetUuid,
    /// Build dependencies will be included in the Builder arguments when building an asset
    pub build_deps: Vec<AssetRef>,
    /// Load dependencies are guaranteed to load before this asset by the Loader
    pub load_deps: Vec<AssetRef>,
    /// Type of compression used to compress this artifact
    pub compression: CompressionType,
    /// Size of this artifact in bytes when compressed
    pub compressed_size: Option<u64>,
    /// Size of this artifact in bytes when serialized and uncompressed
    pub uncompressed_size: Option<u64>,
    /// The UUID of the artifact's Rust type
    pub type_id: AssetTypeId,
}
/// Version of the SourceMetadata struct.
/// Used for forward compatibility to enable changing the .meta file format
pub const SOURCEMETADATA_VERSION: u32 = 1;

/// SourceMetadata is the in-memory representation of the .meta file for a (source, .meta) pair.
#[derive(Serialize, Deserialize)]
pub struct SourceMetadata<Options: 'static, State: 'static> {
    /// Metadata struct version
    pub version: u32,
    /// Hash of the source file + importer options + importer state when last importing source file.
    pub import_hash: Option<u64>,
    /// The [Importer::version] used to import the source file.
    pub importer_version: u32,
    /// The [type_uuid::TypeUuid::UUID] used to import the source file.
    #[serde(default)]
    pub importer_type: AssetTypeId,
    /// The [Importer::Options] used to import the source file.
    pub importer_options: Options,
    /// The [Importer::State] generated when importing the source file.
    pub importer_state: State,
    /// Metadata for assets generated when importing the source file.
    pub assets: Vec<AssetMetadata>,
}

/// Trait object wrapper for [Importer] implementations.
/// Enables using Importers without knowing the concrete type.
/// See [Importer] for documentation on fields.
pub trait BoxedImporter: TypeUuidDynamic + Send + Sync + 'static {
    fn import_boxed<'a>(
        &'a self,
        source: &'a mut (dyn AsyncRead + Unpin + Send + Sync),
        options: Box<dyn SerdeObj>,
        state: Box<dyn SerdeObj>,
    ) -> BoxFuture<'a, Result<BoxedImporterValue>>;
    fn export_boxed<'a>(
        &'a self,
        output: &'a mut (dyn AsyncWrite + Unpin + Send + Sync),
        options: Box<dyn SerdeObj>,
        state: Box<dyn SerdeObj>,
        assets: Vec<ExportAsset>,
    ) -> BoxFuture<'a, Result<BoxedExportInputs>>;
    fn default_options(&self) -> Box<dyn SerdeObj>;
    fn default_state(&self) -> Box<dyn SerdeObj>;
    fn version(&self) -> u32;
    fn deserialize_metadata<'a>(
        &self,
        bytes: &'a [u8]
    ) -> Result<SourceMetadata<Box<dyn SerdeObj>, Box<dyn SerdeObj>>>;
    fn deserialize_options<'a>(
        &self,
        bytes: &'a [u8]
    ) -> Result<Box<dyn SerdeObj>>;
    fn deserialize_state<'a>(
        &self,
        bytes: &'a [u8]
    ) -> Result<Box<dyn SerdeObj>>;
}

impl std::fmt::Debug for dyn BoxedImporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoxedImporter").field(&self.uuid()).finish()
    }
}

/// Trait object wrapper for [ImporterValue] implementations.
/// See [ImporterValue] for documentation on fields.
pub struct BoxedImporterValue {
    pub value: ImporterValue,
    pub options: Box<dyn SerdeObj>,
    pub state: Box<dyn SerdeObj>,
}

/// Return value for BoxedImporter::export_boxed
pub struct BoxedExportInputs {
    pub options: Box<dyn SerdeObj>,
    pub state: Box<dyn SerdeObj>,
    pub value: ImporterValue,
}

impl<S, O, T> BoxedImporter for T
where
    O: SerdeObj + Serialize + Default + Send + Sync + Clone + for<'a> Deserialize<'a>,
    S: SerdeObj + Serialize + Default + Send + Sync + for<'a> Deserialize<'a>,
    T: AsyncImporter<State = S, Options = O> + TypeUuid + Send + Sync,
{
    fn import_boxed<'a>(
        &'a self,
        source: &'a mut (dyn AsyncRead + Unpin + Send + Sync),
        options: Box<dyn SerdeObj>,
        state: Box<dyn SerdeObj>,
    ) -> BoxFuture<'a, Result<BoxedImporterValue>> {
        log::trace!("import_boxed");
        Box::pin(async move {
            let s = state.downcast::<S>();
            let mut s = if let Ok(s) = s {
                s
            } else {
                panic!("Failed to downcast Importer::State");
            };
            let o = options.downcast::<O>();
            let o = if let Ok(o) = o {
                *o
            } else {
                panic!("Failed to downcast Importer::Options");
            };

            log::trace!("import_boxed about to import");
            let result = self.import(source, o.clone(), &mut s).await?;
            log::trace!("import_boxed imported");
            Ok(BoxedImporterValue {
                value: result,
                options: Box::new(o),
                state: s,
            })
        })
    }

    fn export_boxed<'a>(
        &'a self,
        output: &'a mut (dyn AsyncWrite + Unpin + Send + Sync),
        options: Box<dyn SerdeObj>,
        state: Box<dyn SerdeObj>,
        assets: Vec<ExportAsset>,
    ) -> BoxFuture<'a, Result<BoxedExportInputs>> {
        Box::pin(async move {
            let s = state.downcast::<S>();
            let mut s = if let Ok(s) = s {
                s
            } else {
                panic!("Failed to downcast Importer::State");
            };
            let o = options.downcast::<O>();
            let o = if let Ok(o) = o {
                *o
            } else {
                panic!("Failed to downcast Importer::Options");
            };

            let result = self.export(output, o.clone(), &mut s, assets).await?;
            Ok(BoxedExportInputs {
                options: Box::new(o),
                state: s,
                value: result,
            })
        })
    }

    fn default_options(&self) -> Box<dyn SerdeObj> {
        Box::new(O::default())
    }

    fn default_state(&self) -> Box<dyn SerdeObj> {
        Box::new(S::default())
    }

    fn version(&self) -> u32 {
        T::version(self)
    }

    fn deserialize_metadata<'a>(
        &self,
        bytes: &'a [u8]
    ) -> Result<SourceMetadata<Box<dyn SerdeObj>, Box<dyn SerdeObj>>> {
        let metadata: SourceMetadata<O, S> = ron::de::from_bytes(&bytes)?;
        Ok(SourceMetadata {
            version: metadata.version,
            import_hash: metadata.import_hash,
            importer_version: metadata.importer_version,
            importer_type: metadata.importer_type,
            importer_options: Box::new(metadata.importer_options),
            importer_state: Box::new(metadata.importer_state),
            assets: metadata.assets.clone(),
        })
    }

    fn deserialize_options<'a>(
        &self,
        bytes: &'a [u8]
    ) -> Result<Box<dyn SerdeObj>> {
        Ok(Box::new(bincode::deserialize::<O>(&bytes).unwrap()))
    }

    fn deserialize_state<'a>(
        &self,
        bytes: &'a [u8]
    ) -> Result<Box<dyn SerdeObj>> {
        Ok(Box::new(bincode::deserialize::<S>(&bytes).unwrap()))
    }
}
