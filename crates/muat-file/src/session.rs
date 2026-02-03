//! File-backed session implementation.

use async_trait::async_trait;
use tracing::{debug, instrument};

use muat_core::repo::{ListRecordsOutput, Record, RecordValue};
use muat_core::traits::Session as SessionTrait;
use muat_core::types::{AtUri, Did, Nsid, PdsUrl};
use muat_core::{AccessToken, RefreshToken, Result};

use crate::pds::FilePds;

/// Session for a file-backed PDS.
#[derive(Debug, Clone)]
pub struct FileSession {
    pds: FilePds,
    did: Did,
    access_token: AccessToken,
}

impl FileSession {
    pub(crate) fn new(pds: FilePds, did: Did, access_token: AccessToken) -> Self {
        Self {
            pds,
            did,
            access_token,
        }
    }

    pub fn from_persisted(pds: FilePds, access_token: AccessToken) -> Result<Self> {
        let (did, _) = FilePds::parse_token(&access_token)?;
        Ok(Self::new(pds, did, access_token))
    }
}

#[async_trait]
impl SessionTrait for FileSession {
    fn did(&self) -> &Did {
        &self.did
    }

    fn pds(&self) -> &PdsUrl {
        self.pds.url()
    }

    fn access_token(&self) -> AccessToken {
        self.access_token.clone()
    }

    fn refresh_token(&self) -> Option<RefreshToken> {
        None
    }

    #[instrument(skip(self), fields(did = %self.did, %collection))]
    async fn list_records(
        &self,
        repo: &Did,
        collection: &Nsid,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRecordsOutput> {
        debug!("Listing records");
        self.pds.ensure_repo_access(&self.access_token, repo)?;
        self.pds
            .store()
            .list_records(repo, collection, limit, cursor)
            .await
    }

    #[instrument(skip(self), fields(did = %self.did, %uri))]
    async fn get_record(&self, uri: &AtUri) -> Result<Record> {
        debug!("Getting record");
        self.pds
            .ensure_repo_access(&self.access_token, uri.repo())?;
        self.pds.store().get_record(uri).await
    }

    #[instrument(skip(self, value), fields(did = %self.did, %collection))]
    async fn create_record(&self, collection: &Nsid, value: &RecordValue) -> Result<AtUri> {
        debug!("Creating record");
        self.pds.ensure_repo_access(&self.access_token, &self.did)?;
        self.pds
            .store()
            .create_record(&self.did, collection, value, None)
            .await
    }

    #[instrument(skip(self), fields(did = %self.did, %uri))]
    async fn delete_record(&self, uri: &AtUri) -> Result<()> {
        debug!("Deleting record");
        self.pds
            .ensure_repo_access(&self.access_token, uri.repo())?;
        self.pds.store().delete_record(uri).await
    }
}
