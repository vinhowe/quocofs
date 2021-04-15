use crate::error::QuocoError;
use crate::error::QuocoError::{NoObjectWithName, ObjectDoesNotExist};
use crate::formats::{Hashes, Names, ReferenceFormat};
use crate::object::fs_accessor::LOCK_FILE_NAME;
use crate::object::{Key, ObjectHash, ObjectId, ObjectSource, QuocoReader, QuocoWriter};
use crate::util::bytes_to_hex_str;
use crate::Result;
use cloud_storage::{Error, Object};
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::str;
use uuid::Uuid;

const OBJECT_MIME_TYPE: &str = "application/octet-stream";

pub struct GoogleStorageObjectAccessor {
    names: Names,
    hashes: Hashes,
    bucket: String,
    key: Key,
    lock: bool,
}

impl GoogleStorageObjectAccessor {
    pub fn open(bucket: &str, config_path: &Path, key: &Key) -> Result<Self> {
        // Sort of a kludge
        std::env::set_var("SERVICE_ACCOUNT", config_path);

        Self::check_no_lock(bucket)?;
        Self::touch_lock(bucket)?;

        let mut accessor = GoogleStorageObjectAccessor {
            names: Names::default(),
            hashes: Hashes::default(),
            bucket: bucket.into(),
            key: *key,
            lock: true,
        };

        Self::load_reference_formats(&mut accessor)?;

        Ok(accessor)
    }

    // pub async fn storage_hub(config_path: &Path) -> Result<Storage> {
    //     // Get an ApplicationSecret instance by some means. It contains the `client_id` and
    //     // `client_secret`, among other things.
    //     let secret: yup_oauth2::ConsoleApplicationSecret =
    //         serde_json::from_reader(BufReader::new(File::open(config_path)?))?;
    //     // Instantiate the authenticator. It will choose a suitable authentication flow for you,
    //     // unless you replace  `None` with the desired Flow.
    //     // Provide your own `AuthenticatorDelegate` to adjust the way it operates and get feedback about
    //     // what's going on. You probably want to bring in your own `TokenStorage` to persist tokens and
    //     // retrieve them from storage.
    //     let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
    //         secret
    //             .installed
    //             .expect("Couldn't load console application secret"),
    //         yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    //     )
    //     .persist_tokens_to_disk(config_path.parent().unwrap().join("tokencache.json"))
    //     .build()
    //     .await?;
    //
    //     Ok(Storage::new(
    //         hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
    //         auth,
    //     ))
    // }

    pub fn unlock(&mut self) -> Result<()> {
        self.delete(LOCK_FILE_NAME)?;
        self.lock = false;
        Ok(())
    }

    // TODO: Work out naming/semantic division between this and check_no_lock
    fn check_lock(&self) -> Result<()> {
        if !self.lock {
            return Err(QuocoError::SessionDisposed);
        }
        Ok(())
    }

    fn get_object_bytes(&self, name: &str) -> Result<Vec<u8>> {
        Object::download_sync(self.bucket.as_str(), name).map_err(QuocoError::from)
    }

    fn modify_unchecked_reader<R: Read + Seek>(&self, name: &str, reader: &mut R) -> Result<()> {
        // Warning: This loads everything into memory before uploading. As far as I can tell, this
        // is a limitation of cloud_storage but I don't know if it's a limitation of Google's API.
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        self.modify_unchecked(name, data)?;
        Ok(())
    }

    fn modify_unchecked(&self, name: &str, data: Vec<u8>) -> Result<()> {
        Object::create_sync(self.bucket.as_str(), data, name, OBJECT_MIME_TYPE)?;
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<()> {
        Object::delete_sync(self.bucket.as_str(), name)?;
        Ok(())
    }

    fn with_name_exists(bucket: &str, name: &str) -> Result<bool> {
        match Object::read_sync(bucket, name) {
            Ok(_) => Ok(false),
            Err(err) => match err {
                Error::Google(ref response) => {
                    if response.errors_has_reason(&cloud_storage::Reason::NotFound) {
                        Ok(false)
                    } else {
                        Err(err.into())
                    }
                }
                _ => Err(err.into()),
            },
        }
    }

    fn load_reference_formats(&mut self) -> Result<()> {
        let names_name = Names::specification().name;
        let hashes_name = Hashes::specification().name;

        if Self::with_name_exists(self.bucket.as_str(), names_name)? {
            self.names
                .load(&mut Cursor::new(self.get_object_bytes(names_name)?))?;
        }
        if Self::with_name_exists(self.bucket.as_str(), hashes_name)? {
            self.hashes
                .load(&mut Cursor::new(self.get_object_bytes(hashes_name)?))?;
        }

        Ok(())
    }

    fn save_reference_format<F: ReferenceFormat>(&self, format: &F) -> Result<()> {
        let object_name = F::specification().name;
        let format_data = Cursor::new(Vec::new());

        let mut writer = QuocoWriter::new(format_data, &self.key);
        format.save(&mut writer)?;

        self.modify_unchecked(object_name, writer.into_inner().into_inner())?;

        Ok(())
    }

    fn touch_lock(bucket: &str) -> Result<()> {
        Object::create_sync(bucket, Vec::new(), LOCK_FILE_NAME, OBJECT_MIME_TYPE)?;

        Ok(())
    }

    fn check_no_lock(bucket: &str) -> Result<()> {
        if let true = Self::with_name_exists(bucket, LOCK_FILE_NAME)? {
            return Err(QuocoError::SessionPathLocked(format!("gs://{}", bucket)));
        }
        Ok(())
    }
}

impl ObjectSource for GoogleStorageObjectAccessor {
    type OutReader = QuocoReader<Cursor<Vec<u8>>>;

    fn object(&mut self, id: &ObjectId) -> Result<Self::OutReader> {
        self.check_lock()?;

        Ok(QuocoReader::new(
            Cursor::new(self.get_object_bytes(&bytes_to_hex_str(id))?),
            &self.key,
        ))
    }

    fn object_exists(&self, id: &ObjectId) -> Result<bool> {
        self.check_lock()?;

        Self::with_name_exists(self.bucket.as_str(), &bytes_to_hex_str(id))
    }

    fn delete_object(&mut self, id: &ObjectId) -> Result<()> {
        self.check_lock()?;

        self.delete(&bytes_to_hex_str(id))
    }

    fn create_object<R: Read + Seek>(&mut self, reader: &mut R) -> Result<ObjectId> {
        self.check_lock()?;

        let new_id = {
            let uuid = Uuid::new_v4();
            *uuid.as_bytes()
        };
        self.modify_unchecked_reader(&bytes_to_hex_str(&new_id), reader)?;
        Ok(new_id)
    }

    fn modify_object<R: Read + Seek>(&mut self, id: &ObjectId, reader: &mut R) -> Result<()> {
        self.check_lock()?;

        // TODO: Is it worth making an extra network call to check if the document doesn't exist?
        self.modify_unchecked_reader(&bytes_to_hex_str(id), reader)
    }

    fn object_hash(&self, id: &ObjectId) -> Result<&ObjectHash> {
        self.check_lock()?;

        match self.hashes.get_hash(id) {
            None => Err(ObjectDoesNotExist(*id)),
            Some(hash) => Ok(hash),
        }
    }

    fn object_id_with_name(&self, name: &str) -> Result<&ObjectId> {
        self.check_lock()?;

        match self.names.get_id(name) {
            None => Err(NoObjectWithName(name.into())),
            Some(name) => Ok(name),
        }
    }

    fn set_object_name(&mut self, id: &[u8; 16], name: &str) -> Result<()> {
        self.check_lock()?;

        self.names.insert(id, name);

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.check_lock()?;

        self.save_reference_format(&self.hashes)?;
        self.save_reference_format(&self.names)?;

        Ok(())
    }
}

impl Drop for GoogleStorageObjectAccessor {
    fn drop(&mut self) {
        self.flush().expect("Failed to flush.");
        self.unlock()
            .expect("Failed to release lock. You may have to release it manually.");
    }
}
