use crate::document::{DocumentHash, DocumentId};
use std::io::Read;

pub trait DocumentAccessor {
    type OutReader: Read;

    fn document(&mut self, id: &DocumentId) -> Option<Self::OutReader>;
    fn document_exists(&self, id: &DocumentId) -> bool;
    fn delete_document(&mut self, id: &DocumentId) -> bool;
    fn create_document<InR: Read>(&mut self, reader: &mut InR) -> Option<DocumentId>;
    fn modify_document<InR: Read>(&mut self, id: &DocumentId, reader: &mut InR) -> bool;
    fn document_hash(&self, id: &DocumentId) -> Option<&DocumentHash>;
    fn document_id_with_name(&self, name: &str) -> Option<&DocumentId>;
    fn set_document_name(&mut self, id: &DocumentId, name: &str) -> bool;
    fn flush(&mut self) -> bool;
}
