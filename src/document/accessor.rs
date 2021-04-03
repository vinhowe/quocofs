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

// trait TempFileDataProvider<Idx> {
//     fn temp_file_for(&mut self, index: Idx) -> io::Result<fs::File>;
//     fn temp_file_exists_for(&self, index: Idx) -> bool;
//
//     fn clear_temp_files(&mut self) -> io::Result<()> {
//
//     }
//     fn close_temp_file(&mut self, index: Idx) -> io::Result<()> {
//         if !self.temp_file_exists_for(index) {
//             return Ok(());
//         }
//
//         let can_shred = is_shred_available();
//
//         Ok(())
//     }
// }
