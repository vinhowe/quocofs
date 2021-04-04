use crate::object::{ObjectHash, ObjectId};
use std::io::Read;

pub trait ObjectSource {
    type OutReader: Read;

    fn object(&mut self, id: &ObjectId) -> Option<Self::OutReader>;
    fn object_exists(&self, id: &ObjectId) -> bool;
    fn delete_object(&mut self, id: &ObjectId) -> bool;
    fn create_object<InR: Read>(&mut self, reader: &mut InR) -> Option<ObjectId>;
    fn modify_object<InR: Read>(&mut self, id: &ObjectId, reader: &mut InR) -> bool;
    fn object_hash(&self, id: &ObjectId) -> Option<&ObjectHash>;
    fn object_id_with_name(&self, name: &str) -> Option<&ObjectId>;
    fn set_object_name(&mut self, id: &ObjectId, name: &str) -> bool;
    fn flush(&mut self) -> bool;
}
