use crate::object::{ObjectHash, ObjectId};
use crate::{ReadSeek, Result};
use std::collections::hash_map;
use std::io::Read;
use std::time::SystemTime;

// pub type BoxedObjectSource = Box<dyn ObjectSource<OutReader = dyn Read + Send> + Send>;
pub type BoxedObjectSource = Box<dyn ObjectSource + Send>;

pub trait ObjectSource {
    fn object(&mut self, id: &ObjectId) -> Result<Box<dyn Read>>;
    fn object_exists(&self, id: &ObjectId) -> Result<bool>;
    fn delete_object(&mut self, id: &ObjectId) -> Result<()>;
    fn create_object(&mut self, reader: &mut Box<dyn ReadSeek>) -> Result<ObjectId>;
    fn modify_object(&mut self, id: &ObjectId, reader: &mut Box<dyn ReadSeek>) -> Result<()>;
    fn object_hash(&self, id: &ObjectId) -> Result<Option<&ObjectHash>>;
    fn object_name(&self, id: &ObjectId) -> Result<Option<&String>>;
    fn object_id_with_name(&self, name: &str) -> Result<Option<&ObjectId>>;
    fn set_object_name(&mut self, id: &ObjectId, name: &str) -> Result<()>;
    fn remove_object_name(&mut self, id: &ObjectId) -> Result<()>;
    fn last_updated(&self) -> &SystemTime;
    fn hashes_ids(&mut self) -> hash_map::Keys<'_, ObjectId, ObjectHash>;
    fn names_ids(&mut self) -> hash_map::Keys<'_, ObjectId, String>;
    fn flush(&mut self) -> Result<()>;
}
