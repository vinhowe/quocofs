use crate::object::{ObjectHash, ObjectId};
use crate::Result;
use std::io::{Read, Seek};

pub trait ObjectSource {
    type OutReader: Read;

    fn object(&mut self, id: &ObjectId) -> Result<Self::OutReader>;
    fn object_exists(&self, id: &ObjectId) -> Result<bool>;
    fn delete_object(&mut self, id: &ObjectId) -> Result<()>;
    fn create_object<InR: Read + Seek>(&mut self, reader: &mut InR) -> Result<ObjectId>;
    fn modify_object<InR: Read + Seek>(&mut self, id: &ObjectId, reader: &mut InR) -> Result<()>;
    fn object_hash(&self, id: &ObjectId) -> Result<&ObjectHash>;
    fn object_id_with_name(&self, name: &str) -> Result<&ObjectId>;
    fn set_object_name(&mut self, id: &ObjectId, name: &str) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}
