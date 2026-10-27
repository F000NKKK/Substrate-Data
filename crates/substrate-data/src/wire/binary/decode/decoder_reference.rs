//! [`Decoder`] reading a reference: its kind and its id, each checked.

use crate::binary::{DecodeError, Decoder};
use crate::model::value::{KindName, RefKey};

impl<'b> Decoder<'b> {
    /// A reference's kind and id, each checked: a malformed one is an error.
    /// A kind is checked, and made an atom, once for each string of the table.
    pub(in crate::wire::binary) fn reference(
        &mut self,
    ) -> Result<(KindName, &'b str), DecodeError> {
        let kind_at = self.i;
        let kind = self.string_index()?;
        let kind = self.kind_of(kind, kind_at)?;
        let id_at = self.i;
        let id = self.string()?;
        if !RefKey::is_valid(id) {
            return Err(DecodeError {
                message: format!("`{id}` isn't a reference's id"),
                offset: id_at,
            });
        }
        Ok((kind, id))
    }

    /// The kind spelled by table string `i`, read at `at`.
    fn kind_of(&mut self, i: usize, at: usize) -> Result<KindName, DecodeError> {
        if let Some(Some(kind)) = self.kinds.get(i) {
            return Ok(*kind);
        }
        let kind = KindName::new(self.strings[i]).map_err(|e| DecodeError {
            message: e.to_string(),
            offset: at,
        })?;
        if self.kinds.len() <= i {
            self.kinds.resize(i + 1, None);
        }
        self.kinds[i] = Some(kind);
        Ok(kind)
    }
}
